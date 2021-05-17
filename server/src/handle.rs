use actor::ControlFlow;
use log::info;
use net::SessionMessage;

use crate::{State, StateMessage};

pub(super) fn handle(_state: &mut State, message: StateMessage) -> ControlFlow {
    match message {
        StateMessage::Session(message) => {
            match message {
                SessionMessage::Connect(identity) => {
                    info!("Client {} connected", identity.name);
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
        StateMessage::Stop(ret) => {
            ret.send(()).unwrap();
            ControlFlow::Stop
        }
    }
}
