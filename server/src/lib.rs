mod address;
mod authenticator;
mod error;
mod handle;
mod identity;
mod message;
mod resolver;
mod server;
mod state;
mod token;
mod vulkan;

pub use crate::vulkan::*;
pub use address::*;
pub use authenticator::*;
pub use error::*;
pub(self) use handle::handle;
pub(self) use identity::Identity;
pub use message::*;
pub use resolver::*;
pub use server::*;
pub(self) use state::State;
pub use token::*;

pub use net::SessionMessage;
pub use quinn::Certificate;

pub const PROTOCOLS: &[&[u8]] = &[b"wosim/0.1"];
pub const SIZE_LIMIT: usize = 4096;
pub const SERVER_ACTOR_PORT: u16 = 1;
