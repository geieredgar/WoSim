use std::error::Error;
use std::fmt::Debug;

use actor::Address;
use serde::{de::DeserializeOwned, Serialize};

use crate::Message;

pub trait Server: Send + Sync + 'static {
    type AuthToken: Serialize + DeserializeOwned + Send;
    type AuthError: Error;
    type Push: Message + Debug;
    type Request: Message + Debug;

    fn authenticate(
        &self,
        client: Address<Self::Push>,
        token: Self::AuthToken,
    ) -> Result<Address<Self::Request>, Self::AuthError>;

    fn token_size_limit() -> usize;
}
