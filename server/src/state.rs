use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use db::{Database, Entry, Len, Object, Tree};
use noise::{NoiseFn, Perlin};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::Push;

pub(super) struct State {
    pub database: Database<World>,
    pub updates: Vec<Update>,
    pub observers: HashMap<Uuid, Observer>,
}

pub struct Observer {
    pub sync_push: mpsc::Sender<Push>,
    pub after_update: usize,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub struct Orientation {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

pub struct World {
    pub positions: db::Vec<Position>,
    pub players: db::Vec<Player>,
    pub player_index: Tree<u128, usize>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Zeroable, Pod)]
#[repr(C)]
pub struct Player {
    pub uuid: u128,
    pub position: Position,
    pub orientation: Orientation,
    pub _padding: [u8; 8],
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Update {
    NewPlayer(Player),
    Player(Uuid, Position, Orientation),
}

impl World {
    pub fn new(database: db::DatabaseRef) -> Self {
        let perlin = Perlin::new();
        let mut positions = db::Vec::new(database.clone());
        {
            let mut positions = positions.write();
            for x in -20..21 {
                for y in -20..21 {
                    for z in -20..21 {
                        let v = perlin.get([x as f64 / 20.0, y as f64 / 20.0, z as f64 / 20.0]);
                        if v >= 0.0 {
                            positions.push(Position {
                                x: x as f32 * 3.0,
                                y: y as f32 * 3.0,
                                z: z as f32 * 3.0,
                            });
                        }
                    }
                }
            }
        }
        let players = db::Vec::new(database.clone());
        let player_index = db::Tree::new(database);
        Self {
            positions,
            players,
            player_index,
        }
    }

    pub fn register_player(&mut self, uuid: Uuid, updates: &mut Vec<Update>) {
        if let Entry::Vacant(entry) = self.player_index.write().entry(&uuid.as_u128()) {
            let player = Player {
                uuid: uuid.as_u128(),
                position: Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                orientation: Orientation {
                    roll: 0.0,
                    pitch: 0.0,
                    yaw: 0.0,
                },
                _padding: [0; 8],
            };
            let mut writer = self.players.write();
            let index = writer.len();
            writer.push(player);
            entry.insert(index);
            updates.push(Update::NewPlayer(player));
        }
    }

    pub fn update_player(
        &mut self,
        uuid: Uuid,
        pos: Position,
        orientation: Orientation,
        updates: &mut Vec<Update>,
    ) {
        let player_index = *self.player_index.read().get(&uuid.as_u128()).unwrap();
        let player = &mut self.players.write()[player_index];
        player.position = pos;
        player.orientation = orientation;
        updates.push(Update::Player(uuid, pos, orientation))
    }
}

impl Object for World {
    fn format() -> db::Format {
        [64; 256]
    }

    fn serialize(&mut self, mut writer: impl std::io::Write) -> std::io::Result<()> {
        self.positions.serialize(&mut writer)?;
        self.players.serialize(&mut writer)?;
        self.player_index.serialize(&mut writer)?;
        Ok(())
    }

    fn deserialize(
        mut reader: impl std::io::Read,
        database: db::DatabaseRef,
    ) -> std::io::Result<Self> {
        let positions = db::Vec::deserialize(&mut reader, database.clone())?;
        let players = db::Vec::deserialize(&mut reader, database.clone())?;
        let player_index = db::Tree::deserialize(&mut reader, database)?;
        Ok(Self {
            positions,
            players,
            player_index,
        })
    }
}
