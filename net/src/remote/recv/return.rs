use bytes::Bytes;
use eyre::eyre;
use log::error;
use quinn::SendStream;
use serde::Serialize;
use tokio::{spawn, sync::mpsc};

use crate::recv::error::ConvertErr;

use super::error::Error;

pub struct Return(ReturnInner);

enum ReturnInner {
    None,
    Unique(SendStream),
    Shared(u32, mpsc::Sender<(u32, Bytes)>),
}

impl Return {
    pub fn r#return<T: Serialize + Send + 'static>(self, value: T) {
        spawn(async move {
            if let Err(Error::Error(error)) = self.send(value).await {
                error!("{:?}", error);
            }
        });
    }

    pub(super) fn none() -> Self {
        Self(ReturnInner::None)
    }

    pub(super) fn unique(tx: SendStream) -> Self {
        Self(ReturnInner::Unique(tx))
    }

    pub(super) fn shared(key: u32, tx: mpsc::Sender<(u32, Bytes)>) -> Self {
        Self(ReturnInner::Shared(key, tx))
    }

    async fn send<T: Serialize>(self, value: T) -> Result<(), Error> {
        let buf = bincode::serialize(&value).convert_err("could not serialize message")?;
        match self.0 {
            ReturnInner::None => Err(Error::Error(eyre!("could not send message"))),
            ReturnInner::Unique(mut tx) => {
                tx.write_all(&buf)
                    .await
                    .convert_err("could not write message data")?;
                tx.finish().await.convert_err("could not finish stream")
            }
            ReturnInner::Shared(key, tx) => tx
                .send((key, buf.into()))
                .await
                .convert_err("could not send message"),
        }
    }
}
