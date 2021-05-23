use std::error::Error;
use std::fmt::Debug;

use actor::Address;

use crate::{Client, Message};

pub trait Server: Send + Sync + 'static {
    type AuthError: Error;
    type Push: Message + Debug;
    type Request: Message + Debug;

    fn authenticate(
        &self,
        client: Client,
        address: Address<Self::Push>,
    ) -> Result<Address<Self::Request>, Self::AuthError>;

    fn token_size_limit() -> usize;
}
