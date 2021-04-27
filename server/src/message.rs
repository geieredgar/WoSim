use std::fmt::Display;

use net::{FromBiStream, FromDatagram, FromUniStream, Message};
use quinn::SendStream;

#[derive(Debug)]
pub struct ServerMessage;

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
pub struct ClientMessage;

impl FromDatagram for ClientMessage {
    type Error = MessageError;

    fn from(_reader: net::Reader) -> Result<Self, Self::Error> {
        Ok(Self)
    }
}

impl FromUniStream for ClientMessage {
    type Error = MessageError;

    fn from(_reader: net::Reader) -> Result<Self, Self::Error> {
        Ok(Self)
    }

    fn size_limit() -> usize {
        4096
    }
}

impl FromBiStream for ClientMessage {
    type Error = MessageError;

    fn from(_reader: net::Reader, _send: SendStream) -> Result<Self, Self::Error> {
        Ok(Self)
    }

    fn size_limit() -> usize {
        4096
    }
}

impl Message for ClientMessage {
    type Error = MessageError;

    fn send(self, _connection: quinn::Connection) -> Result<(), MessageError> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum MessageError {}

impl std::error::Error for MessageError {}

impl Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
