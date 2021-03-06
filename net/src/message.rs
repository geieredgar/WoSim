use std::{
    error::Error,
    fmt::{Debug, Display},
};

use bincode::{deserialize, serialize_into};
use bytes::{BufMut, Bytes, BytesMut};
use log::error;
use serde::{de::DeserializeOwned, Serialize};
use tokio::{
    spawn,
    sync::oneshot::{self},
};

use crate::{recv, Return};

pub trait Message: Sync + Send + Debug + Sized + 'static {
    fn into_outgoing(self) -> Result<OutgoingMessage, MessageError>;

    fn from_incoming(message: IncomingMessage) -> Result<Self, MessageError>;

    fn size_limit(message_id: u32) -> usize;
}

pub type MessageError = Box<dyn Error + Send + Sync + 'static>;

pub struct IncomingMessage {
    id: u32,
    buf: Bytes,
    r#return: recv::Return,
}

pub struct OutgoingMessage {
    pub(super) packet: Bytes,
    pub(super) ty: MessageType,
}

#[derive(Debug)]
pub struct FailedOutgoingMessage;

#[derive(Debug)]
pub struct InvalidIncomingId(u32);

pub(super) enum MessageType {
    Datagram,
    Uni,
    Bi(oneshot::Sender<Bytes>, usize),
}

impl IncomingMessage {
    pub(super) fn new(id: u32, buf: Bytes, r#return: recv::Return) -> Self {
        Self { id, buf, r#return }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn value<T: DeserializeOwned>(&self) -> Result<T, bincode::Error> {
        deserialize(&self.buf)
    }

    pub fn into_return<T>(self) -> Return<T> {
        Return::Remote(self.r#return)
    }

    pub fn invalid_id_error(&self) -> MessageError {
        Box::new(InvalidIncomingId(self.id))
    }
}

impl OutgoingMessage {
    fn new<T: Serialize>(id: u32, value: T, ty: MessageType) -> Result<Self, bincode::Error> {
        assert_ne!(id, 0);
        let mut packet = BytesMut::new();
        packet.put_u32(id);
        let mut packet = packet.writer();
        serialize_into(&mut packet, &value)?;
        Ok(Self {
            packet: packet.into_inner().freeze(),
            ty,
        })
    }

    pub fn datagram<T: Serialize>(id: u32, value: T) -> Result<Self, bincode::Error> {
        Self::new(id, value, MessageType::Datagram)
    }

    pub fn uni<T: Serialize>(id: u32, value: T) -> Result<Self, bincode::Error> {
        Self::new(id, value, MessageType::Uni)
    }

    pub fn bi<T: Serialize, U: Serialize + DeserializeOwned + Send + 'static>(
        id: u32,
        value: T,
        ret: Return<U>,
        ret_size_limit: usize,
    ) -> Result<Self, bincode::Error> {
        let (send, recv) = oneshot::channel();
        spawn(async move {
            let buf: Bytes = match recv.await {
                Ok(buf) => buf,
                Err(error) => {
                    error!("{:?}", error);
                    return;
                }
            };
            let value = match deserialize(&buf) {
                Ok(value) => value,
                Err(error) => {
                    error!("{:?}", error);
                    return;
                }
            };
            let _ = ret.send(value);
        });
        Self::new(id, value, MessageType::Bi(send, ret_size_limit))
    }

    pub fn fail() -> Result<OutgoingMessage, Box<dyn Error>> {
        Err(Box::new(FailedOutgoingMessage))
    }
}

impl Display for FailedOutgoingMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not create outgoing message")
    }
}

impl Error for FailedOutgoingMessage {}

impl Display for InvalidIncomingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid incoming id {}", self.0)
    }
}

impl Error for InvalidIncomingId {}
