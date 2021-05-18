use std::{fs::read, io, net::SocketAddr, path::Path, sync::Arc};

use crate::{handle, state::World, Authenticator, Error, State, StateMessage, PROTOCOLS};
use actor::{mailbox, Address, ControlFlow};
use db::Database;
use net::listen;
use quinn::{
    Certificate, CertificateChain, Endpoint, PrivateKey, ServerConfig, ServerConfigBuilder,
    TransportConfig, VarInt,
};
use tokio::{spawn, sync::oneshot};

pub struct Server {
    endpoint: Option<Endpoint>,
    pub(super) authenticator: Arc<Authenticator>,
    root_address: Address<StateMessage>,
}

impl Server {
    pub fn new() -> io::Result<Self> {
        let (mut mailbox, root_address) = mailbox();
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
        let address = root_address.clone().map(StateMessage::Session);
        let authenticator = Arc::new(Authenticator::new(address));
        Ok(Self {
            endpoint: None,
            authenticator,
            root_address,
        })
    }

    pub fn open(&mut self, addr: &SocketAddr) -> Result<(), Error> {
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
        server_config.protocols(PROTOCOLS);
        server_config.certificate(cert_chain, key)?;
        let server_config = server_config.build();
        let mut endpoint = Endpoint::builder();
        endpoint.listen(server_config);
        let (endpoint, incoming) = endpoint.bind(&addr)?;
        listen(incoming, self.authenticator.clone());
        self.endpoint = Some(endpoint);
        Ok(())
    }

    pub async fn stop(&mut self) {
        self.close();
        let (send, recv) = oneshot::channel();
        self.root_address.send(StateMessage::Stop(send));
        recv.await.unwrap()
    }

    pub fn close(&mut self) {
        if let Some(endpoint) = self.endpoint.take() {
            endpoint.close(VarInt::from_u32(2), "Server closed".as_bytes());
        }
    }
}
