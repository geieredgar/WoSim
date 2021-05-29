use std::{io, net::SocketAddr, sync::Arc};

use libmdns::Responder;
use quinn::{Endpoint, EndpointError, ServerConfig, ServerConfigBuilder, VarInt};
use rustls::TLSError;
use thiserror::Error;
use tokio::spawn;

use crate::Service;

use super::listen;

pub struct Listener {
    endpoint: Endpoint,
    _service: libmdns::Service,
}

#[derive(Debug, Error)]
pub enum OpenError {
    #[error("invalid certificate / private key pair")]
    InvalidCertificate(#[source] TLSError),
    #[error("could not bind to address")]
    Bind(#[source] EndpointError),
    #[error("could not get local address")]
    LocalAddress(#[source] io::Error),
}

impl Listener {
    pub fn open<S: Service>(service: Arc<S>, address: &SocketAddr) -> Result<Self, OpenError> {
        let mut server_config = ServerConfig::default();
        server_config.transport = Arc::new(S::server_transport_config());
        let mut server_config = ServerConfigBuilder::new(server_config);
        let protocol = S::protocol();
        server_config.protocols(&[protocol.as_bytes()]);
        server_config
            .certificate(service.certificate_chain(), service.private_key())
            .map_err(OpenError::InvalidCertificate)?;
        let server_config = server_config.build();
        let mut endpoint = Endpoint::builder();
        endpoint.listen(server_config);
        let (endpoint, incoming) = endpoint.bind(address).map_err(OpenError::Bind)?;
        let port = endpoint
            .local_addr()
            .map_err(OpenError::LocalAddress)?
            .port();
        let (responder, task) = Responder::with_default_handle().unwrap();
        spawn(listen(incoming, service.clone()));
        spawn(task);
        let service_type = S::service_type();
        let _service = responder.register(
            format!("_{}._udp", service_type),
            format!("{}-{}", service_type, port),
            port,
            &[
                protocol,
                service.authentication_type(),
                service.name(),
                service.description(),
            ],
        );
        Ok(Self { endpoint, _service })
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.endpoint
            .close(VarInt::from_u32(1), "Server closed".as_bytes());
    }
}
