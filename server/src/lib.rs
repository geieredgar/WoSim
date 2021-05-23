mod address;
mod error;
mod handle;
mod identity;
mod message;
mod resolver;
mod server;
mod state;
mod vulkan;

use std::io;

pub use crate::vulkan::*;
pub use address::*;
use db::Database;
pub use error::*;
pub(self) use handle::handle;
pub(self) use identity::Identity;
pub use message::*;
pub use resolver::*;
pub use server::*;
pub(self) use state::State;
pub(self) use state::World;

pub use net::Connection;
pub use quinn::Certificate;

pub const PROTOCOL: &str = "wosim/0.1";

pub fn create_world() -> io::Result<()> {
    let mut db = Database::create("world.db", World::new)?;
    db.snapshot()?;
    Ok(())
}
