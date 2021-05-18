use std::error::Error;
use std::fmt::Debug;

use actor::Address;
use serde::{de::DeserializeOwned, Serialize};

use crate::Message;

pub trait Authenticator: Send + Sync + 'static {
    type Token: Serialize + DeserializeOwned + Send;
    type Error: Error;
    type ClientMessage: Message + Debug;
    type ServerMessage: Message + Debug;

    fn authenticate(
        &self,
        client: Address<Self::ClientMessage>,
        token: Self::Token,
    ) -> Result<Address<Self::ServerMessage>, Self::Error>;

    fn token_size_limit() -> usize;
}
