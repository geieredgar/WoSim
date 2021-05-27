use std::{
    collections::{hash_map::Entry, HashMap},
    io::{self, ErrorKind},
};

use actor::Address;
use bincode::serialize_into;
use bytemuck::{Pod, Zeroable};
use db::{Database, Object};
use noise::{NoiseFn, Perlin};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Push;

pub(super) struct State {
    pub database: Database<World>,
    pub updates: Vec<Update>,
    pub observers: HashMap<Uuid, Observer>,
}

pub struct Observer {
    pub sync_push: Address<Push>,
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
    pub players: HashMap<Uuid, Player>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub position: Position,
    pub orientation: Orientation,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Update {
    NewPlayer(Uuid, Player),
    Player(Uuid, Position, Orientation),
}

impl World {
    pub fn new(database: db::DatabaseRef) -> Self {
        let perlin = Perlin::new();
        let mut positions = db::Vec::new(database);
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
        Self {
            positions,
            players: HashMap::new(),
        }
    }

    pub fn register_player(&mut self, uuid: Uuid, updates: &mut Vec<Update>) {
        if let Entry::Vacant(entry) = self.players.entry(uuid) {
            let player = Player {
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
            };
            entry.insert(player.clone());
            updates.push(Update::NewPlayer(uuid, player));
        }
    }

    pub fn update_player(
        &mut self,
        uuid: Uuid,
        pos: Position,
        orientation: Orientation,
        updates: &mut Vec<Update>,
    ) {
        let player = self.players.get_mut(&uuid).unwrap();
        player.position = pos;
        player.orientation = orientation;
        updates.push(Update::Player(uuid, pos, orientation))
    }
}

impl Object for World {
    fn format() -> db::Format {
        [64; 256]
    }

    fn serialize(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.positions.serialize(writer)?;
        serialize_into(writer, &self.players)
            .map_err(|error| io::Error::new(ErrorKind::Other, error))?;
        Ok(())
    }

    fn deserialize(
        reader: &mut impl std::io::Read,
        database: db::DatabaseRef,
    ) -> std::io::Result<Self> {
        let positions = db::Vec::deserialize(reader, database)?;
        let players = bincode::deserialize_from(reader)
            .map_err(|error| io::Error::new(ErrorKind::Other, error))?;
        Ok(Self { positions, players })
    }
}
