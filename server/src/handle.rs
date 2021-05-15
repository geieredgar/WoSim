use actor::ControlFlow;
use log::info;
use net::SessionMessage;

use crate::{Identity, ServerMessage, State};

pub(super) fn handle(
    _state: &mut State,
    message: SessionMessage<Identity, ServerMessage>,
) -> ControlFlow {
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
    ControlFlow::Continue
}
