use std::{
    error::Error,
    fmt::Display,
    fs::read,
    io,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{handle, state::World, Identity, Push, Request, ServerMessage, State, PROTOCOL};
use actor::{mailbox, Address, ControlFlow};
use db::Database;
use net::{listen, Client};
use quinn::{
    Certificate, CertificateChain, Endpoint, PrivateKey, ServerConfig, ServerConfigBuilder,
    TransportConfig, VarInt,
};
use tokio::{spawn, sync::oneshot};

#[derive(Debug)]
pub struct Server {
    endpoint: Mutex<Option<Endpoint>>,
    address: Address<ServerMessage>,
}

impl Server {
    pub fn new() -> io::Result<Arc<Self>> {
        let (mut mailbox, address) = mailbox();
        let path = Path::new("world.db");
        let database = if path.exists() {
            Database::open("world.db")?
        } else {
            Database::create("world.db", World::new)?
        };
        spawn(async move {
            let mut state = State { database };
            while let Some(message) = mailbox.recv().await {
                if let ControlFlow::Stop = handle(&mut state, message).await {
                    return;
                }
            }
        });
        Ok(Arc::new(Self {
            endpoint: Mutex::new(None),
            address,
        }))
    }

    pub fn open(self: &Arc<Self>, addr: &SocketAddr) -> Result<(), crate::Error> {
        self.close();
        let pem = read("key.pem")?;
        let key = PrivateKey::from_pem(&pem)?;
        let pem = read("cert.pem")?;
        let cert = Certificate::from_pem(&pem)?;
        let cert_chain = CertificateChain::from_certs(vec![cert]);
        let mut server_config = ServerConfig::default();
        let transport_config = TransportConfig::default();
        server_config.transport = Arc::new(transport_config);
        let mut server_config = ServerConfigBuilder::new(server_config);
        server_config.protocols(&[PROTOCOL.as_bytes()]);
        server_config.certificate(cert_chain, key)?;
        let server_config = server_config.build();
        let mut endpoint = Endpoint::builder();
        endpoint.listen(server_config);
        let (endpoint, incoming) = endpoint.bind(&addr)?;
        listen(incoming, self.clone());
        *self.endpoint.lock().unwrap() = Some(endpoint);
        Ok(())
    }

    pub async fn stop(&self) {
        self.close();
        let (send, recv) = oneshot::channel();
        self.address.send(ServerMessage::Stop(send));
        recv.await.unwrap()
    }

    pub fn close(&self) {
        let mut endpoint = self.endpoint.lock().unwrap();
        if let Some(endpoint) = endpoint.take() {
            endpoint.close(VarInt::from_u32(2), "Server closed".as_bytes());
        }
    }
}

impl net::Server for Server {
    type AuthError = AuthenticationError;
    type Push = Push;
    type Request = Request;

    fn authenticate(
        &self,
        client: Client,
        address: Address<Self::Push>,
    ) -> Result<Address<Self::Request>, Self::AuthError> {
        let identity = Identity {
            name: match client {
                Client::Local => "root",
                Client::Remote { token } => token,
            }
            .to_owned(),
            address,
        };
        let (mut mailbox, address) = mailbox();
        {
            let address = self.address.clone();
            spawn(async move {
                address.send(ServerMessage::Connected(identity.clone()));
                {
                    while let Some(message) = mailbox.recv().await {
                        address.send(ServerMessage::Request(identity.clone(), message));
                    }
                }
                address.send(ServerMessage::Disconnected(identity));
            });
        }
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
