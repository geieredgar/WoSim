use std::fmt::Display;

use net::{FromBiStream, FromDatagram, FromUniStream, Message, ReadError, SessionMessage, Writer};
use quinn::SendStream;
use tokio::{spawn, sync::oneshot};

use crate::{state::Position, Identity};

#[derive(Debug)]
pub struct ServerMessage;

#[derive(Debug)]
pub(crate) enum StateMessage {
    Session(SessionMessage<Identity, ServerMessage>),
    Stop(oneshot::Sender<()>),
}

impl FromDatagram for ServerMessage {
    type Error = MessageError;

    fn from(_reader: net::Reader) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl FromUniStream for ServerMessage {
    type Error = MessageError;

    fn from(_reader: net::Reader) -> Result<Self, Self::Error> {
        Ok(Self)
    }

    fn size_limit() -> usize {
        4096
    }
}

impl FromBiStream for ServerMessage {
    type Error = MessageError;

    fn from(_reader: net::Reader, _send: SendStream) -> Result<Self, Self::Error> {
        Ok(Self)
    }

    fn size_limit() -> usize {
        4096
    }
}

impl Message for ServerMessage {
    type Error = MessageError;

    fn send(self, _connection: quinn::Connection) -> Result<(), MessageError> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum ClientMessage {
    Positions(Vec<Position>),
}

impl FromDatagram for ClientMessage {
    type Error = MessageError;

    fn from(_reader: net::Reader) -> Result<Self, Self::Error> {
        Err(MessageError::Invalid)
    }
}

impl FromUniStream for ClientMessage {
    type Error = MessageError;

    fn from(mut reader: net::Reader) -> Result<Self, Self::Error> {
        let positions = reader.read()?;
        Ok(Self::Positions(positions))
    }

    fn size_limit() -> usize {
        1024 * 1024
    }
}

impl FromBiStream for ClientMessage {
    type Error = MessageError;

    fn from(_reader: net::Reader, _send: SendStream) -> Result<Self, Self::Error> {
        Err(MessageError::Invalid)
    }

    fn size_limit() -> usize {
        0
    }
}

impl Message for ClientMessage {
    type Error = MessageError;

    fn send(self, connection: quinn::Connection) -> Result<(), MessageError> {
        match self {
            ClientMessage::Positions(positions) => spawn(async move {
                let send = connection.open_uni().await.unwrap();
                let mut writer = Writer::new();
                writer.write(&positions).unwrap();
                writer.send(send).await.unwrap();
            }),
        };
        Ok(())
    }
}

#[derive(Debug)]
pub enum MessageError {
    Read(ReadError),
    Invalid,
}

impl std::error::Error for MessageError {}

impl Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ReadError> for MessageError {
    fn from(error: ReadError) -> Self {
        Self::Read(error)
    }
}
