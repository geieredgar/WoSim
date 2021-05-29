use std::ops::{Deref, DerefMut};

use bytes::{Buf, Bytes, BytesMut};
use eyre::eyre;
use tokio::sync::oneshot;

use crate::{Message, MessageType};

use super::{
    error::{ConvertErr, Error},
    Connection, Return,
};

pub(super) struct Stream<T>(pub(super) T, pub(super) Connection);

pub(super) type SendStream = Stream<quinn::SendStream>;
pub(super) type RecvStream = Stream<quinn::RecvStream>;

pub(super) struct BiStream(pub(super) SendStream, pub(super) RecvStream);

impl<T> Deref for Stream<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Stream<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl SendStream {
    pub(super) async fn send_message<M: Message>(
        &mut self,
        message: M,
        r#return: &Return,
        key: u32,
    ) -> Result<(), Error> {
        let message = message
            .into_outgoing()
            .convert_err("could not convert message")?;
        match message.ty {
            MessageType::Datagram => self.write_packet(message.packet).await,
            MessageType::Uni => self.write_packet(message.packet).await,
            MessageType::Bi(sender, size_limit) => {
                r#return.wait(key, sender, size_limit);
                self.write_packet(message.packet).await
            }
        }
    }

    pub(super) async fn write_packet(&mut self, mut packet: Bytes) -> Result<(), Error> {
        self.write_all(&packet.get_u32().to_be_bytes())
            .await
            .convert_err("could not write message id")?;
        self.write_all(&(packet.remaining() as u32).to_be_bytes())
            .await
            .convert_err("could not write message length")?;
        self.write_all(&packet)
            .await
            .convert_err("could not write message data")
    }

    pub(super) async fn write_packet_and_finish(mut self, packet: Bytes) -> Result<(), Error> {
        self.write_packet(packet).await?;
        self.finish().await.convert_err("could not finish stream")
    }
}

impl RecvStream {
    pub(super) async fn recv_message(&mut self, size_limit: usize) -> Result<Bytes, Error> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)
            .await
            .convert_err("could not read message size")?;
        let size = u32::from_be_bytes(buf) as usize;
        if size > size_limit {
            return Err(Error::Error(eyre!(
                "message is larger than allowed: {} > {}",
                size,
                size_limit
            )));
        }
        let mut buf = BytesMut::with_capacity(size);
        unsafe { buf.set_len(size) };
        self.read_exact(&mut buf)
            .await
            .convert_err("could not read message data")?;
        Ok(buf.freeze())
    }

    pub(super) async fn recv_messages(mut self, r#return: Return) -> Result<(), Error> {
        loop {
            let mut buf = [0; 4];
            self.read_exact(&mut buf)
                .await
                .convert_err("could not read message key")?;
            let key = u32::from_be_bytes(buf);
            if key == 0 {
                return Ok(());
            }
            if let Some((tx, size_limit)) = r#return.wake(key) {
                tx.send(self.recv_message(size_limit).await?)
                    .map_err(|_| Error::Error(eyre!("receiver already closed")))?;
            } else {
                return Err(Error::Error(eyre!("invalid message key {}", key)));
            }
        }
    }
}

impl BiStream {
    pub(super) fn split(self) -> (SendStream, RecvStream) {
        (self.0, self.1)
    }

    pub(super) async fn handle_packet_and_response(
        mut self,
        packet: Bytes,
        tx: oneshot::Sender<Bytes>,
        size_limit: usize,
    ) -> Result<(), Error> {
        self.0.write_packet_and_finish(packet).await?;
        tx.send(self.1.recv_message(size_limit).await?)
            .map_err(|_| Error::Error(eyre!("receiver already closed")))
    }
}
