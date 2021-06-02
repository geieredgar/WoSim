use std::{net::SocketAddr, sync::Arc};

use crate::{
    recv::{Listener, OpenError},
    Service,
};

pub struct Server<S: Service> {
    service: Arc<S>,
    address: SocketAddr,
    _listener: Option<Listener>,
}

impl<S: Service> Server<S> {
    pub fn new(service: Arc<S>, address: SocketAddr) -> Self {
        Self {
            service,
            address,
            _listener: None,
        }
    }

    pub fn open(&mut self, use_mdns: bool) -> Result<(), OpenError> {
        self._listener = Some(Listener::open(
            self.service.clone(),
            &self.address,
            use_mdns,
        )?);
        Ok(())
    }

    pub fn close(&mut self) {
        self._listener = None
    }

    pub fn service(&self) -> &Arc<S> {
        &self.service
    }
}
