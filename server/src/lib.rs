mod error;
mod handle;
mod message;
mod service;
mod state;
mod user;
mod vulkan;

use std::io;

pub use crate::vulkan::*;
use db::Database;
pub use error::*;
pub(self) use handle::handle;
pub use message::*;
pub use service::*;
pub use state::Orientation;
pub use state::Player;
pub use state::Position;
pub(self) use state::State;
pub use state::Update;
pub(self) use state::World;
pub(self) use user::User;

pub use net::Connection;
pub use quinn::Certificate;

pub const PROTOCOL: &str = "wosim/0.1";

pub fn create_world() -> io::Result<()> {
    let mut db = Database::create("world.db", World::new)?;
    db.snapshot()?;
    Ok(())
}
