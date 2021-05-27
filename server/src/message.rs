use std::{collections::HashMap, sync::Arc};

use net::{Message, OutgoingMessage};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::{
    state::{Position, Update},
    Player, User,
};

#[derive(Debug)]
pub enum Request {
    UpdatePosition(Position),
}

#[derive(Debug)]
pub(crate) enum ServerMessage {
    Connected(User),
    Disconnected(User),
    Request(User, Request),
    Stop(oneshot::Sender<()>),
    PushUpdates,
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
    Setup(Setup),
    Updates(UpdateBatch),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Setup(pub Uuid, pub HashMap<Uuid, Player>, pub Vec<Position>);

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateBatch(pub Arc<Vec<Update>>, pub usize);

impl Message for Push {
    fn into_outgoing(self) -> Result<net::OutgoingMessage, Box<dyn std::error::Error>> {
        match self {
            Push::Setup(setup) => Ok(OutgoingMessage::uni(1, setup)?),
            Push::Updates(updates) => Ok(OutgoingMessage::uni(2, updates)?),
        }
    }

    fn from_incoming(message: net::IncomingMessage) -> Result<Self, Box<dyn std::error::Error>> {
        match message.id() {
            1 => Ok(Self::Setup(message.value()?)),
            2 => Ok(Self::Updates(message.value()?)),
            _ => message.invalid_id(),
        }
    }

    fn size_limit(_message_id: u32) -> usize {
        4096 * 4096
    }
}
