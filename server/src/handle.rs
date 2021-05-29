use std::{mem::swap, sync::Arc};

use actor::ControlFlow;
use log::info;

use crate::{state::Observer, Push, SelfUpdate, ServerMessage, Setup, State, UpdateBatch, World};

pub(super) async fn handle(state: &mut State, message: ServerMessage) -> ControlFlow {
    match message {
        ServerMessage::Stop(ret) => {
            state.database.snapshot().unwrap();
            ret.send(()).unwrap();
            return ControlFlow::Stop;
        }
        ServerMessage::Connected(user) => {
            let world: &mut World = &mut state.database;
            world.register_player(user.uuid, &mut state.updates);
            let positions = world.positions.read().iter().cloned().collect();
            info!("User {} connected", user.name);
            let observer = Observer {
                sync_push: user.connection.synchronous(),
                after_update: state.updates.len(),
            };
            let _ = observer
                .sync_push
                .send(Push::Setup(Setup(
                    user.uuid,
                    world.players.clone(),
                    positions,
                )))
                .await;
            state.observers.insert(user.uuid, observer);
        }
        ServerMessage::Disconnected(user) => {
            info!("User {} disconnected", user.name);
            state.observers.remove(&user.uuid);
        }
        ServerMessage::PushUpdates => {
            let mut updates = Vec::new();
            swap(&mut state.updates, &mut updates);
            let updates = Arc::new(updates);
            for (_, observer) in state.observers.iter_mut() {
                let _ = observer
                    .sync_push
                    .send(Push::Updates(UpdateBatch(
                        updates.clone(),
                        observer.after_update,
                    )))
                    .await;
                observer.after_update = 0;
            }
        }
        ServerMessage::Request(user, request) => match request {
            crate::Request::UpdateSelf(SelfUpdate(pos, orientation)) => {
                let world: &mut World = &mut state.database;
                world.update_player(user.uuid, pos, orientation, &mut state.updates)
            }
            crate::Request::Shutdown => panic!(),
        },
    }
    ControlFlow::Continue
}
