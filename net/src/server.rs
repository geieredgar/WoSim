use std::{net::SocketAddr, sync::Arc};

use crate::{Listener, OpenError, Service};

pub struct Server<S: Service> {
    service: Arc<S>,
    _listener: Option<Listener>,
}

impl<S: Service> Server<S> {
    pub fn new(service: Arc<S>) -> Self {
        Self {
            service,
            _listener: None,
        }
    }

    pub fn open(&mut self, address: &SocketAddr) -> Result<(), OpenError> {
        self._listener = Some(Listener::open(self.service.clone(), address)?);
        Ok(())
    }

    pub fn close(&mut self) {
        self._listener = None
    }
}
