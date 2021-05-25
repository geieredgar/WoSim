use bincode::serialize;
use bytes::Bytes;
use log::error;
use quinn::SendStream;
use serde::Serialize;
use tokio::{spawn, sync::mpsc::UnboundedSender};

use crate::SendError;

pub enum Sender {
    None,
    Unique(SendStream),
    Shared(u32, UnboundedSender<(u32, Bytes)>),
}

impl Sender {
    pub fn send<T: Serialize + Send + 'static>(self, value: T) {
        spawn(async move {
            if let Err(error) = self.async_send(value).await {
                error!("{}", error);
            }
        });
    }

    async fn async_send<T: Serialize>(self, value: T) -> Result<(), SendError> {
        let buf = serialize(&value).map_err(SendError::SerializeResponse)?;
        match self {
            Sender::None => return Err(SendError::NoResponseSender),
            Sender::Unique(mut send) => {
                send.write_all(&buf)
                    .await
                    .map_err(SendError::WriteResponseData)?;
                send.finish().await.map_err(SendError::FinishResponse)?;
            }
            Sender::Shared(id, send) => send
                .send((id, buf.into()))
                .map_err(SendError::SendResponsePair)?,
        }
        Ok(())
    }
}
