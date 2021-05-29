use std::{ops::Deref, sync::Arc};

use bytes::Bytes;
use log::error;
use quinn::{SendDatagramError, VarInt};
use quinn_proto::ConnectionStats;
use tokio::{spawn, sync::mpsc};

use crate::{Message, MessageType};

use super::{
    error::{ConvertErr, Error},
    stream::{BiStream, SendStream, Stream},
    Return,
};

const CHANNEL_BUFFER: usize = 16;

#[derive(Clone, Debug)]
pub struct Connection(Arc<ConnectionInner>);

#[derive(Debug)]
struct ConnectionInner(quinn::Connection);

impl Deref for ConnectionInner {
    type Target = quinn::Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Connection {
    pub fn new(connection: quinn::Connection) -> Self {
        Self(Arc::new(ConnectionInner(connection)))
    }

    pub fn into_asynchronous<M: Message>(self) -> mpsc::Sender<M> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);
        spawn(async move {
            if let Err(Error::Error(error)) = self.handle_asynchronous_messages(rx).await {
                error!("{:?}", error)
            }
        });
        tx
    }

    pub fn into_synchronous<M: Message>(self) -> mpsc::Sender<M> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);
        spawn(async move {
            if let Err(Error::Error(error)) = self.handle_synchronous_messages(rx).await {
                error!("{:?}", error)
            }
        });
        tx
    }

    pub fn stats(&self) -> ConnectionStats {
        self.0.stats()
    }

    async fn open_uni(&self) -> Result<SendStream, Error> {
        let tx = self.0.open_uni().await?;
        Ok(Stream(tx, self.clone()))
    }

    async fn open_bi(&self) -> Result<BiStream, Error> {
        let (tx, rx) = self.0.open_bi().await?;
        Ok(BiStream(Stream(tx, self.clone()), Stream(rx, self.clone())))
    }

    fn send_datagram(&self, data: Bytes) -> Result<(), SendDatagramError> {
        self.0.send_datagram(data)
    }

    async fn handle_asynchronous_messages<M: Message>(
        self,
        mut rx: mpsc::Receiver<M>,
    ) -> Result<(), Error> {
        while let Some(message) = rx.recv().await {
            self.send_message(message)
                .await
                .convert_err("could not send message")?;
        }
        Ok(())
    }

    async fn send_message<M: Message>(&self, message: M) -> Result<(), Error> {
        let message = message
            .into_outgoing()
            .convert_err("could not convert message")?;
        match message.ty {
            MessageType::Datagram => {
                self.send_datagram(message.packet)
                    .convert_err("could not send datagram")?;
            }
            MessageType::Uni => {
                let tx = self.open_uni().await.convert_err("could not open stream")?;
                spawn(async move {
                    if let Err(Error::Error(error)) =
                        tx.write_packet_and_finish(message.packet).await
                    {
                        error!("{:?}", error)
                    }
                });
            }
            MessageType::Bi(tx, size_limit) => {
                let stream = self.open_bi().await.convert_err("could not open stream")?;
                let packet = message.packet;
                spawn(async move {
                    if let Err(Error::Error(error)) = stream
                        .handle_packet_and_response(packet, tx, size_limit)
                        .await
                    {
                        error!("{:?}", error)
                    }
                });
            }
        };
        Ok(())
    }

    async fn handle_synchronous_messages<M: Message>(
        self,
        mut rx: mpsc::Receiver<M>,
    ) -> Result<(), Error> {
        let r#return = Return::default();
        let mut tx = {
            let (tx, rx) = self
                .open_bi()
                .await
                .convert_err("could not open stream")?
                .split();
            let r#return = r#return.clone();
            spawn(async move {
                if let Err(Error::Error(error)) = rx.recv_messages(r#return).await {
                    error!("{:?}", error)
                }
            });
            tx
        };
        tx.write_all(&0u32.to_be_bytes())
            .await
            .convert_err("could not write multi-message begin marker")?;
        let mut key = 1;
        while let Some(message) = rx.recv().await {
            tx.send_message(message, &r#return, key)
                .await
                .convert_err("could not send message")?;
            key += 1;
        }
        tx.write_all(&0u32.to_be_bytes())
            .await
            .convert_err("could not write multi-message end marker")?;
        tx.finish().await.convert_err("could not finish stream")
    }
}

impl Drop for ConnectionInner {
    fn drop(&mut self) {
        self.0.close(VarInt::from_u32(0), &[]);
    }
}
