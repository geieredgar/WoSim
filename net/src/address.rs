use actor::Address;
use log::{error, warn};
use quinn::{Connection, SendStream};
use serde::Serialize;
use tokio::{spawn, sync::oneshot::Sender};

use crate::Message;

pub enum ReturnAddress<T: Serialize + Send + 'static> {
    Local(Sender<T>),
    Remote(SendStream),
}

impl<T: Serialize + Send + 'static> ReturnAddress<T> {
    pub fn send(self, message: T) {
        match self {
            ReturnAddress::Local(ret) => {
                if ret.send(message).is_err() {
                    warn!("Could not return value. Receiver already dropped");
                }
            }
            ReturnAddress::Remote(mut send) => {
                spawn(async move {
                    let data = match bincode::serialize(&message) {
                        Ok(data) => data,
                        Err(error) => {
                            error!("Serializing return value failed: {}", error);
                            return;
                        }
                    };
                    if let Err(error) = send.write_all(&data).await {
                        warn!("Writing return value to stream failed: {}", error);
                        return;
                    }
                    if let Err(error) = send.finish().await {
                        warn!("Shutting down stream failed: {}", error)
                    }
                });
            }
        }
    }
}

impl<T: Serialize + Send + 'static> From<Sender<T>> for ReturnAddress<T> {
    fn from(sender: Sender<T>) -> Self {
        ReturnAddress::Local(sender)
    }
}

pub fn remote_address<M: Message>(connection: Connection) -> Address<M> {
    Address::new(move |message: M| {
        let connection = connection.clone();
        spawn(async move {
            if let Err(error) = message.send(connection) {
                warn!("Sending message failed: {}", error)
            }
        });
        Ok(())
    })
}
