use std::{error::Error, fmt::Display};

use actor::{mailbox, Address};
use net::SessionMessage;
use tokio::spawn;

use crate::{ClientMessage, Identity, ServerMessage, Token};

pub(super) struct Authenticator {
    server: Address<SessionMessage<Identity, ServerMessage>>,
}

impl Authenticator {
    pub(super) fn new(server: Address<SessionMessage<Identity, ServerMessage>>) -> Self {
        Self { server }
    }
}

impl net::Authenticator for Authenticator {
    type Token = Token;
    type ClientMessage = ClientMessage;
    type ServerMessage = ServerMessage;
    type Error = AuthenticationError;

    fn authenticate(
        &self,
        client: actor::Address<Self::ClientMessage>,
        token: Self::Token,
    ) -> Result<actor::Address<Self::ServerMessage>, Self::Error> {
        let (mut mailbox, address) = mailbox();
        let identity = Identity {
            name: token.name,
            address: client,
        };
        let server = self.server.clone();
        spawn(async move {
            server.send(SessionMessage::Connect(identity.clone()));
            {
                while let Some(message) = mailbox.recv().await {
                    server.send(SessionMessage::Message(identity.clone(), message));
                }
            }
            server.send(SessionMessage::Disconnect(identity));
        });
        Ok(address)
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
