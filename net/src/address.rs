use log::{error, warn};
use quinn::SendStream;
use serde::Serialize;
use tokio::{spawn, sync::oneshot::Sender};

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
