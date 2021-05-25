use std::{
    env::current_dir,
    error::Error,
    fmt::{Debug, Display},
    io,
    time::Duration,
};

use crate::{handle, Identity, Push, Request, ServerMessage, State, PROTOCOL};
use actor::{mailbox, Address, ControlFlow, SendError};
use db::Database;
use log::error;
use net::{AuthToken, Connection};
use quinn::{Certificate, CertificateChain, ParseError, PrivateKey, TransportConfig};
use rcgen::{generate_simple_self_signed, RcgenError};
use tokio::{spawn, sync::oneshot};

pub struct Service {
    name: String,
    certificate_chain: CertificateChain,
    private_key: PrivateKey,
    address: Address<ServerMessage>,
}

#[derive(Debug)]
pub enum CreateServiceError {
    NoCurrentDir(io::Error),
    CurrentDirIsRootDir,
    OpenDatabase(io::Error),
    GenerateCertificates(RcgenError),
    ParsePrivateKey(ParseError),
    SerializeCertificate(RcgenError),
    ParseCertificate(ParseError),
}

impl Service {
    pub fn new() -> Result<Self, CreateServiceError> {
        let path = current_dir().map_err(CreateServiceError::NoCurrentDir)?;
        let name = path
            .file_name()
            .ok_or(CreateServiceError::CurrentDirIsRootDir)?
            .to_string_lossy()
            .to_string();
        let (mut mailbox, address) = mailbox();
        let database = Database::open("world.db").map_err(CreateServiceError::OpenDatabase)?;
        spawn(async move {
            let mut state = State { database };
            while let Some(message) = mailbox.recv().await {
                if let ControlFlow::Stop = handle(&mut state, message).await {
                    return;
                }
            }
        });
        let cert = generate_simple_self_signed(["localhost".to_owned()])
            .map_err(CreateServiceError::GenerateCertificates)?;
        let der = cert.serialize_private_key_der();
        let private_key =
            PrivateKey::from_der(&der).map_err(CreateServiceError::ParsePrivateKey)?;
        let der = cert
            .serialize_der()
            .map_err(CreateServiceError::SerializeCertificate)?;
        let cert = Certificate::from_der(&der).map_err(CreateServiceError::ParseCertificate)?;
        let certificate_chain = CertificateChain::from_certs(vec![cert]);
        Ok(Self {
            name,
            certificate_chain,
            private_key,
            address,
        })
    }

    pub async fn stop(&self) -> Result<(), SendError> {
        let (send, recv) = oneshot::channel();
        self.address.send(ServerMessage::Stop(send))?;
        recv.await.unwrap();
        Ok(())
    }
}

impl net::Service for Service {
    type AuthError = AuthenticationError;
    type Push = Push;
    type Request = Request;

    fn authenticate(
        &self,
        connection: Connection<Self::Push>,
        token: AuthToken,
    ) -> Result<Address<Self::Request>, Self::AuthError> {
        let identity = Identity {
            name: match token {
                AuthToken::Local(token) => token,
                AuthToken::Remote(token) => token,
            }
            .to_owned(),
            connection,
        };
        let (mut mailbox, address) = mailbox();
        {
            let address = self.address.clone();
            spawn(async move {
                if let Err(error) = address.send(ServerMessage::Connected(identity.clone())) {
                    error!("{}", error);
                    return;
                }
                {
                    while let Some(message) = mailbox.recv().await {
                        if let Err(error) =
                            address.send(ServerMessage::Request(identity.clone(), message))
                        {
                            error!("{}", error);
                            return;
                        }
                    }
                }
                if let Err(error) = address.send(ServerMessage::Disconnected(identity)) {
                    error!("{}", error)
                }
            });
        }
        Ok(address)
    }

    fn token_size_limit(&self) -> usize {
        4096
    }

    fn service_type() -> &'static str {
        "wosim-server"
    }

    fn protocol() -> &'static str {
        PROTOCOL
    }

    fn authentication_type(&self) -> &str {
        "none"
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "A local world"
    }

    fn certificate_chain(&self) -> CertificateChain {
        self.certificate_chain.clone()
    }

    fn private_key(&self) -> PrivateKey {
        self.private_key.clone()
    }

    fn client_transport_config() -> TransportConfig {
        let mut config = TransportConfig::default();
        config.keep_alive_interval(Some(Duration::from_secs(5)));
        config
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
