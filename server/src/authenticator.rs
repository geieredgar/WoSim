use std::{error::Error, fmt::Display};

use crate::{ClientMessage, Identity, Token};

#[derive(Default)]
pub(super) struct Authenticator {}

impl Authenticator {
    pub(super) fn new() -> Self {
        Self {}
    }
}

impl net::Authenticator for Authenticator {
    type Token = Token;
    type Identity = Identity;
    type Error = AuthenticationError;

    type ClientMessage = ClientMessage;

    fn authenticate(
        &self,
        client: actor::Address<Self::ClientMessage>,
        token: Self::Token,
    ) -> Result<Self::Identity, Self::Error> {
        Ok(Identity {
            name: token.name,
            address: client,
        })
    }

    fn token_size_limit() -> usize {
        4096
    }
}

#[derive(Debug)]
pub struct AuthenticationError {}

impl Display for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Authentication failed")
    }
}

impl Error for AuthenticationError {}
