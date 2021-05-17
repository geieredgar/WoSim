use bytemuck::{Pod, Zeroable};
use db::{Database, Object};
use serde::{Deserialize, Serialize};

pub(super) struct State {
    pub database: Database<World>,
}

#[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct World {
    pub positions: db::Vec<Position>,
}

impl World {
    pub fn new(database: db::DatabaseRef) -> Self {
        let mut positions = db::Vec::new(database);
        {
            let mut positions = positions.write();
            for x in -20..21 {
                for y in -20..21 {
                    for z in -20..21 {
                        positions.push(Position {
                            x: x as f32 * 3.0,
                            y: y as f32 * 3.0,
                            z: z as f32 * 3.0,
                        });
                    }
                }
            }
        }
        Self { positions }
    }
}

impl Object for World {
    fn format() -> db::Format {
        [64; 256]
    }

    fn serialize(&mut self, writer: &mut impl std::io::Write) -> std::io::Result<()> {
        self.positions.serialize(writer)
    }

    fn deserialize(
        reader: &mut impl std::io::Read,
        database: db::DatabaseRef,
    ) -> std::io::Result<Self> {
        let positions = db::Vec::deserialize(reader, database)?;
        Ok(Self { positions })
    }
}
