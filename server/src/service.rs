use std::{
    collections::HashMap, env::current_dir, fmt::Debug, io, string::FromUtf8Error, sync::Mutex,
    time::Duration,
};

use crate::{handle, ControlFlow, Push, Request, ServerMessage, State, User, PROTOCOL};
use base64::DecodeError;
use db::Database;
use log::error;
use net::{AuthToken, Connection};
use quinn::{Certificate, CertificateChain, ParseError, PrivateKey, TransportConfig};
use rcgen::{generate_simple_self_signed, RcgenError};
use thiserror::Error;
use tokio::{spawn, sync::mpsc, task::JoinHandle, time::interval};
use uuid::Uuid;

const CHANNEL_BOUND: usize = 16;

pub struct Service {
    name: String,
    certificate_chain: CertificateChain,
    private_key: PrivateKey,
    tx: mpsc::Sender<ServerMessage>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Debug, Error)]
pub enum CreateServiceError {
    #[error("could not find current working directory")]
    NoCurrentDir(#[source] io::Error),
    #[error("cannot create service in root directory")]
    CurrentDirIsRootDir,
    #[error("could not open database")]
    OpenDatabase(#[source] io::Error),
    #[error("could not generate self-signed certificates")]
    GenerateCertificates(#[source] RcgenError),
    #[error("could not parse private key")]
    ParsePrivateKey(#[source] ParseError),
    #[error("could not serialize certificate")]
    SerializeCertificate(#[source] RcgenError),
    #[error("could not parse certificate")]
    ParseCertificate(#[source] ParseError),
}

impl Service {
    pub fn new() -> Result<Self, CreateServiceError> {
        let path = current_dir().map_err(CreateServiceError::NoCurrentDir)?;
        let name = path
            .file_name()
            .ok_or(CreateServiceError::CurrentDirIsRootDir)?
            .to_string_lossy()
            .to_string();
        let (tx, mut rx) = mpsc::channel(CHANNEL_BOUND);
        let database = Database::open("world.db").map_err(CreateServiceError::OpenDatabase)?;
        let handle = Mutex::new(Some(spawn(async move {
            let mut state = State {
                database,
                updates: Vec::new(),
                observers: HashMap::new(),
            };
            while let Some(message) = rx.recv().await {
                if let ControlFlow::Stop = handle(&mut state, message).await {
                    return;
                }
            }
        })));
        {
            let tx = tx.clone();
            spawn(async move {
                let mut interval = interval(Duration::from_millis(1000 / 30));
                loop {
                    interval.tick().await;
                    if tx.send(ServerMessage::PushUpdates).await.is_err() {
                        break;
                    }
                }
            });
        }
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
            tx,
            handle,
        })
    }

    pub async fn stop(&self) {
        self.tx.send(ServerMessage::Stop).await.unwrap();
        self.handle.lock().unwrap().take().unwrap().await.unwrap();
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
    ) -> Result<mpsc::Sender<Self::Request>, Self::AuthError> {
        let token = match token {
            AuthToken::Local(token) => token,
            AuthToken::Remote(token) => token,
        };
        let mut split = token.split('#');
        let uuid = if let Some(uuid) = split.next() {
            Uuid::parse_str(uuid).map_err(AuthenticationError::ParseUuid)?
        } else {
            return Err(AuthenticationError::EmptyToken);
        };
        let name = if let Some(username) = split.next() {
            String::from_utf8(
                base64::decode(username).map_err(AuthenticationError::DecodeUsername)?,
            )
            .map_err(AuthenticationError::IllformedUsername)?
        } else {
            return Err(AuthenticationError::MissingTokenSeparator);
        };
        let user = User {
            uuid,
            name,
            connection,
        };
        let (tx, mut rx) = mpsc::channel(CHANNEL_BOUND);
        {
            let tx = self.tx.clone();
            spawn(async move {
                if let Err(error) = tx.send(ServerMessage::Connected(user.clone())).await {
                    error!("{}", error);
                    return;
                }
                while let Some(request) = rx.recv().await {
                    if let Request::Shutdown = request {
                        break;
                    }
                    if let Err(error) = tx.send(ServerMessage::Request(user.clone(), request)).await
                    {
                        error!("{}", error);
                        return;
                    }
                }
                if let Err(error) = tx.send(ServerMessage::Disconnected(user)).await {
                    error!("{}", error)
                }
            });
        }
        Ok(tx)
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

#[derive(Debug, Error)]
pub enum AuthenticationError {
    #[error("separator '#' is missing")]
    MissingTokenSeparator,
    #[error("could not parse uuid")]
    ParseUuid(#[source] uuid::Error),
    #[error("could not decode username")]
    DecodeUsername(#[source] DecodeError),
    #[error("username is no valid UTF-8")]
    IllformedUsername(#[source] FromUtf8Error),
    #[error("token is empty")]
    EmptyToken,
}
