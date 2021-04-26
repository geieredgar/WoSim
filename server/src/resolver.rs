use std::{io, net::ToSocketAddrs, sync::Arc, time::Duration};

use actor::Address;
use net::{local_connect, remote_connect, EstablishConnectionError, SessionMessage};
use quinn::{Certificate, ClientConfigBuilder, Endpoint, EndpointError, TransportConfig};

use crate::{ClientMessage, ServerAddress, ServerMessage, Token, PROTOCOLS};

pub struct Resolver {
    certificates: Vec<Certificate>,
}

impl Resolver {
    pub fn new(certificates: Vec<Certificate>) -> Self {
        Self { certificates }
    }

    pub async fn resolve<
        F: FnOnce(Address<ServerMessage>) -> Address<SessionMessage<(), ClientMessage>>,
    >(
        &self,
        factory: F,
        server: ServerAddress<'_>,
        token: Token,
    ) -> Result<
        (
            Address<SessionMessage<(), ClientMessage>>,
            Address<ServerMessage>,
        ),
        ResolveError,
    > {
        match server {
            ServerAddress::Local(server) => local_connect(
                server.address.clone(),
                server.authenticator.as_ref(),
                factory,
                (),
                token,
            )
            .map_err(ResolveError::EstablishConnection),
            ServerAddress::Remote(address) => {
                let mut endpoint = Endpoint::builder();
                let mut client_config = ClientConfigBuilder::default();
                client_config.protocols(PROTOCOLS);
                for certificate in self.certificates.iter() {
                    client_config
                        .add_certificate_authority(certificate.clone())
                        .map_err(ResolveError::CertificateAuthority)?;
                }
                let mut client_config = client_config.build();
                let mut transport_config = TransportConfig::default();
                transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
                client_config.transport = Arc::new(transport_config);
                endpoint.default_client_config(client_config);
                let (endpoint, _) = endpoint
                    .bind(&"[::]:0".parse().unwrap())
                    .map_err(ResolveError::Bind)?;
                let mut split = address.splitn(2, ':');
                let hostname = match split.next() {
                    Some(host) => host,
                    None => &address,
                };
                let port = match split.next() {
                    Some(port) => port,
                    None => "8888",
                };
                let address = format!("{}:{}", hostname, port);
                let address = address
                    .to_socket_addrs()
                    .map_err(ResolveError::IpResolve)?
                    .next()
                    .ok_or(ResolveError::NoSocketAddr)?;
                remote_connect(&endpoint, &address, hostname, factory, (), &token)
                    .await
                    .map_err(ResolveError::EstablishConnection)
            }
        }
    }
}

#[derive(Debug)]
pub enum ResolveError {
    CertificateAuthority(webpki::Error),
    Bind(EndpointError),
    IpResolve(io::Error),
    NoSocketAddr,
    EstablishConnection(EstablishConnectionError),
    NewClientMessage,
    Login(String),
}
