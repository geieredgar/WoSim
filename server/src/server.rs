use std::{fs::read, io, net::SocketAddr, path::Path, sync::Arc};

use crate::{
    handle, state::World, Authenticator, Error, Identity, ServerMessage, State, StateMessage,
    PROTOCOLS,
};
use actor::{mailbox, Actor, Address};
use db::Database;
use net::{listen, SessionMessage};
use quinn::{
    Certificate, CertificateChain, Endpoint, PrivateKey, ServerConfig, ServerConfigBuilder,
    TransportConfig, VarInt,
};
use tokio::{spawn, sync::oneshot};

pub struct Server {
    endpoint: Option<Endpoint>,
    pub(super) authenticator: Arc<Authenticator>,
    root_address: Address<StateMessage>,
    pub(super) address: Address<SessionMessage<Identity, ServerMessage>>,
}

impl Server {
    pub fn new() -> io::Result<Self> {
        let authenticator = Arc::new(Authenticator::new());
        let (mailbox, root_address) = mailbox();
        let path = Path::new("world.db");
        let database = if path.exists() {
            Database::open("world.db")?
        } else {
            Database::create("world.db", World::new)?
        };
        let mut state = State { database };
        let handler = move |message| handle(&mut state, message);
        spawn(async move {
            let mut actor = Actor::new(mailbox, handler);
            actor.run().await;
        });
        let address = root_address.clone().map(StateMessage::Session);
        Ok(Self {
            endpoint: None,
            authenticator,
            root_address,
            address,
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
        listen(incoming, self.authenticator.clone(), self.address.clone());
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
