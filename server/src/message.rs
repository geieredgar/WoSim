use net::{Message, OutgoingMessage};
use tokio::sync::oneshot;

use crate::{state::Position, User};

#[derive(Debug)]
pub struct Request;

#[derive(Debug)]
pub(crate) enum ServerMessage {
    Connected(User),
    Disconnected(User),
    Request(User, Request),
    Stop(oneshot::Sender<()>),
}

impl Message for Request {
    fn into_outgoing(self) -> Result<net::OutgoingMessage, Box<dyn std::error::Error>> {
        OutgoingMessage::fail()
    }

    fn from_incoming(message: net::IncomingMessage) -> Result<Self, Box<dyn std::error::Error>> {
        message.invalid_id()
    }

    fn size_limit(_message_id: u32) -> usize {
        0
    }
}

#[derive(Debug)]
pub enum Push {
    Positions(Vec<Position>),
}

impl Message for Push {
    fn into_outgoing(self) -> Result<net::OutgoingMessage, Box<dyn std::error::Error>> {
        match self {
            Push::Positions(positions) => Ok(OutgoingMessage::uni(1, positions)?),
        }
    }

    fn from_incoming(message: net::IncomingMessage) -> Result<Self, Box<dyn std::error::Error>> {
        match message.id() {
            1 => Ok(Self::Positions(message.value()?)),
            _ => message.invalid_id(),
        }
    }

    fn size_limit(_message_id: u32) -> usize {
        4096 * 4096
    }
}
