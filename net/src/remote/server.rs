use std::{net::SocketAddr, sync::Arc};

use quinn::{CertificateChain, PrivateKey};

use crate::{
    recv::{Listener, OpenError},
    Service,
};

pub struct ServerConfiguration {
    pub address: SocketAddr,
    pub certificate_chain: CertificateChain,
    pub private_key: PrivateKey,
    pub use_mdns: bool,
}

pub struct Server<S: Service> {
    service: Arc<S>,
    configuration: ServerConfiguration,
    _listener: Option<Listener>,
}

impl<S: Service> Server<S> {
    pub fn new(service: Arc<S>, configuration: ServerConfiguration) -> Self {
        Self {
            service,
            configuration,
            _listener: None,
        }
    }

    pub fn open(&mut self) -> Result<(), OpenError> {
        self._listener = Some(Listener::open(self.service.clone(), &self.configuration)?);
        Ok(())
    }

    pub fn close(&mut self) {
        self._listener = None
    }

    pub fn service(&self) -> &Arc<S> {
        &self.service
    }
}
