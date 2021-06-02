use std::sync::Arc;

use net::{Message, OutgoingMessage};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    state::{Orientation, Position, Update},
    Player, User,
};

#[derive(Debug)]
pub enum Request {
    UpdateSelf(SelfUpdate),
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SelfUpdate(pub Position, pub Orientation);

#[derive(Debug)]
pub(crate) enum ServerMessage {
    Connected(User),
    Disconnected(User),
    Request(User, Request),
    Stop,
    PushUpdates,
}

impl Message for Request {
    fn into_outgoing(
        self,
    ) -> Result<net::OutgoingMessage, Box<dyn std::error::Error + Send + Sync + 'static>> {
        match self {
            Request::UpdateSelf(value) => Ok(OutgoingMessage::datagram(1, value)?),
            Request::Shutdown => Ok(OutgoingMessage::uni(2, ())?),
        }
    }

    fn from_incoming(
        message: net::IncomingMessage,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        match message.id() {
            1 => Ok(Self::UpdateSelf(message.value()?)),
            2 => Ok(Self::Shutdown),
            _ => Err(message.invalid_id_error()),
        }
    }

    fn size_limit(_message_id: u32) -> usize {
        0
    }
}

#[derive(Debug)]
pub enum Push {
    Setup(Setup),
    Updates(UpdateBatch),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Setup(pub Uuid, pub Vec<Player>, pub Vec<Position>);

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateBatch(pub Arc<Vec<Update>>, pub usize);

impl Message for Push {
    fn into_outgoing(
        self,
    ) -> Result<net::OutgoingMessage, Box<dyn std::error::Error + Send + Sync + 'static>> {
        match self {
            Push::Setup(setup) => Ok(OutgoingMessage::uni(1, setup)?),
            Push::Updates(updates) => Ok(OutgoingMessage::uni(2, updates)?),
        }
    }

    fn from_incoming(
        message: net::IncomingMessage,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
        match message.id() {
            1 => Ok(Self::Setup(message.value()?)),
            2 => Ok(Self::Updates(message.value()?)),
            _ => Err(message.invalid_id_error()),
        }
    }

    fn size_limit(_message_id: u32) -> usize {
        4096 * 4096
    }
}
