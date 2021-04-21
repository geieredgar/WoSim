use std::sync::Arc;

use actor::{Address, Return};
use log::{error, warn};
use quinn::SendStream;
use serde::Serialize;
use tokio::spawn;

use crate::{Connection, HostBinding, Protocol};

pub struct NetAddress<T: Protocol>(Address<T>, Option<Arc<HostBinding>>);

impl<T: Protocol> NetAddress<T> {
    pub(super) fn new(address: Address<T>, binding: Option<HostBinding>) -> Self {
        Self(address, binding.map(Arc::new))
    }

    pub fn port(&self, connection: &Connection) -> u16 {
        if let Some(binding) = self.1.as_ref() {
            binding.port(connection)
        } else {
            panic!("Address not bound to any port");
        }
    }

    pub fn send(&self, message: T) {
        self.0.send(message)
    }
}

impl<T: Protocol> Clone for NetAddress<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

pub enum ReturnAddress<T: Serialize + Send + 'static> {
    Local(Return<T>),
    Remote(SendStream, Connection),
}

impl<T: Serialize + Send + 'static> ReturnAddress<T> {
    pub fn send(self, message: T) {
        match self {
            ReturnAddress::Local(ret) => {
                if ret.send(message).is_err() {
                    warn!("Could not return value. Receiver already dropped");
                }
            }
            ReturnAddress::Remote(mut send, connection) => {
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
                    drop(connection)
                });
            }
        }
    }
}

impl<T: Serialize + Send + 'static> From<Return<T>> for ReturnAddress<T> {
    fn from(ret: Return<T>) -> Self {
        ReturnAddress::Local(ret)
    }
}
