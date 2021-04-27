use std::error::Error;

use actor::Address;
use serde::{de::DeserializeOwned, Serialize};

use crate::Message;

pub trait Authenticator: Send + Sync + 'static {
    type Token: Serialize + DeserializeOwned + Send;
    type Identity: Clone + Send + Sync;
    type Error: Error;
    type ClientMessage: Message;

    fn authenticate(
        &self,
        client: Address<Self::ClientMessage>,
        token: Self::Token,
    ) -> Result<Self::Identity, Self::Error>;

    fn token_size_limit() -> usize;
}
