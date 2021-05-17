use actor::ControlFlow;
use log::info;
use net::SessionMessage;

use crate::{ClientMessage, State, StateMessage, World};

pub(super) fn handle(state: &mut State, message: StateMessage) -> ControlFlow {
    match message {
        StateMessage::Session(message) => {
            match message {
                SessionMessage::Connect(identity) => {
                    let world: &World = &state.database;
                    let positions = world.positions.read().iter().cloned().collect();
                    info!("Client {} connected", identity.name);
                    identity.address.send(ClientMessage::Positions(positions));
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
            state.database.snapshot().unwrap();
            ret.send(()).unwrap();
            ControlFlow::Stop
        }
    }
}
