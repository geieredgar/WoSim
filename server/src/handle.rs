use actor::ControlFlow;
use log::info;

use crate::{Push, ServerMessage, State, World};

pub(super) async fn handle(state: &mut State, message: ServerMessage) -> ControlFlow {
    match message {
        ServerMessage::Stop(ret) => {
            state.database.snapshot().unwrap();
            ret.send(()).unwrap();
            return ControlFlow::Stop;
        }
        ServerMessage::Connected(identity) => {
            let world: &World = &state.database;
            let positions = world.positions.read().iter().cloned().collect();
            info!("Client {} connected", identity.name);
            let _ = identity
                .connection
                .parallel()
                .send(Push::Positions(positions));
        }
        ServerMessage::Disconnected(identity) => {
            info!("Client {} disconnected", identity.name);
        }
        ServerMessage::Request(identity, _) => {
            info!("Request from client {}", identity.name);
        }
    }
    ControlFlow::Continue
}
