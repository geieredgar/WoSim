use std::{io, net::ToSocketAddrs, sync::Arc, time::Duration};

use actor::{Address, Mailbox};
use net::{local_connect, remote_connect, Connection, EstablishConnectionError};
use quinn::{Certificate, ClientConfigBuilder, Endpoint, EndpointError, TransportConfig};

use crate::{Push, Request, Server, ServerAddress, PROTOCOL};

pub struct Resolver {
    certificates: Vec<Certificate>,
}

impl Resolver {
    pub fn new(certificates: Vec<Certificate>) -> Self {
        Self { certificates }
    }

    pub async fn resolve(
        &self,
        server: ServerAddress,
    ) -> Result<(Address<Request>, Mailbox<Push>, Connection<Server>), ResolveError> {
        Ok(match server {
            ServerAddress::Local => {
                let server = Server::new().map_err(ResolveError::LocalSetupFailed)?;
                local_connect(server).map_err(ResolveError::EstablishConnection)?
            }
            ServerAddress::Remote { address, token } => {
                let mut endpoint = Endpoint::builder();
                let mut client_config = ClientConfigBuilder::default();
                client_config.protocols(&[PROTOCOL.as_bytes()]);
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
                let hostname = split.next().unwrap_or(&address);
                let port = split.next().unwrap_or("8888");
                let address = format!("{}:{}", hostname, port);
                let address = address
                    .to_socket_addrs()
                    .map_err(ResolveError::IpResolve)?
                    .next()
                    .ok_or(ResolveError::NoSocketAddr)?;
                remote_connect(&endpoint, &address, hostname, &token)
                    .await
                    .map_err(ResolveError::EstablishConnection)?
            }
        })
    }
}

#[derive(Debug)]
pub enum ResolveError {
    CertificateAuthority(webpki::Error),
    Bind(EndpointError),
    LocalSetupFailed(io::Error),
    IpResolve(io::Error),
    NoSocketAddr,
    EstablishConnection(EstablishConnectionError),
    NewClientMessage,
    Login(String),
}
