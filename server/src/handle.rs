use std::sync::{Arc, Mutex};

use log::info;
use net::SessionMessage;

use crate::{Identity, ServerMessage, State};

pub(super) async fn handle(
    _state: Arc<Mutex<State>>,
    message: SessionMessage<Identity, ServerMessage>,
) {
    match message {
        SessionMessage::Connect(identity) => {
            info!("Client {} connected", identity.name)
        }
        SessionMessage::Disconnect(identity) => {
            info!("Client {} disconnected", identity.name)
        }
        SessionMessage::Message(identity, _) => {
            info!("Message from client {}", identity.name)
        }
    }
}
