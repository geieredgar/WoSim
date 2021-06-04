use std::{
    error::Error,
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs},
    str::FromStr,
    sync::Arc,
};

use quinn::{
    ClientConfigBuilder, ConnectError, ConnectionError, Endpoint, EndpointError, NewConnection,
    WriteError,
};
use thiserror::Error;
use tokio::{spawn, sync::mpsc};

use crate::{
    local_server_address, recv, self_signed, send, AuthToken, Connection, SelfSignError, Server,
    ServerConfiguration, Service, Verification,
};

const CHANNEL_BUFFER: usize = 16;

pub enum Resolver<S: Service> {
    Local {
        service: Arc<S>,
        token: String,
        port: u16,
    },
    Remote {
        hostname: String,
        port: u16,
        token: String,
        verification: Verification,
    },
}

pub type ResolveResult<S> = Result<ResolveSuccess<S>, ResolveError<<S as Service>::AuthError>>;

pub type ResolveSuccess<S> = (
    Connection<<S as Service>::Request>,
    mpsc::Receiver<<S as Service>::Push>,
    Option<Server<S>>,
);
#[derive(Debug, Error)]
pub enum ResolveError<A: Error + 'static> {
    #[error("could not authenticate token")]
    TokenAuthentication(#[source] A),
    #[error("certificate authority has invalid certificate")]
    InvalidCaCertificates(#[source] webpki::Error),
    #[error("could not bind to endpoint")]
    Bind(#[source] EndpointError),
    #[error("could not resolve ip address")]
    IpResolution(#[source] io::Error),
    #[error("could not find a socket address")]
    NoSocketAddrFound,
    #[error("could not connect to server")]
    Connect(#[source] ConnectError),
    #[error("could not connect to server")]
    Connecting(#[source] ConnectionError),
    #[error("could not open stream for to send token")]
    OpenTokenStream(#[source] ConnectionError),
    #[error("could not write to token stream")]
    WriteTokenStream(#[source] WriteError),
    #[error("could not finish token stream")]
    FinishTokenStream(#[source] WriteError),
    #[error("could not generate self-signed certificate")]
    SelfSign(#[from] SelfSignError),
}

impl<S: Service> Resolver<S> {
    pub async fn resolve(self) -> ResolveResult<S> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);
        match self {
            Resolver::Local {
                service,
                token,
                port,
            } => {
                let tx = service
                    .authenticate(Connection::local(tx), AuthToken::Local(&token))
                    .map_err(ResolveError::TokenAuthentication)?;
                let (certificate_chain, private_key) = self_signed()?;
                Ok((
                    Connection::local(tx),
                    rx,
                    Some(Server::new(
                        service,
                        ServerConfiguration {
                            address: local_server_address(port),
                            private_key,
                            certificate_chain,
                            use_mdns: true,
                        },
                    )),
                ))
            }
            Resolver::Remote {
                hostname,
                port,
                token,
                verification,
            } => {
                let mut client_config = ClientConfigBuilder::default();
                client_config.protocols(&[S::protocol().as_bytes()]);
                let mut client_config = verification
                    .apply(client_config)
                    .map_err(ResolveError::InvalidCaCertificates)?;
                client_config.transport = Arc::new(S::client_transport_config());
                let mut endpoint = Endpoint::builder();
                endpoint.default_client_config(client_config);
                let remote_address = (hostname.as_str(), port)
                    .to_socket_addrs()
                    .map_err(ResolveError::IpResolution)?
                    .next()
                    .ok_or(ResolveError::NoSocketAddrFound)?;
                let local_address = match remote_address {
                    SocketAddr::V4(_) => {
                        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
                    }
                    SocketAddr::V6(_) => {
                        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0))
                    }
                };
                let (endpoint, _) = endpoint.bind(&local_address).map_err(ResolveError::Bind)?;
                let server_name = match IpAddr::from_str(&hostname) {
                    Ok(_) => "localhost",
                    Err(_) => &hostname,
                };
                let NewConnection {
                    connection,
                    bi_streams,
                    uni_streams,
                    datagrams,
                    ..
                } = endpoint
                    .connect(&remote_address, server_name)
                    .map_err(ResolveError::Connect)?
                    .await
                    .map_err(ResolveError::Connecting)?;
                let mut send = connection
                    .open_uni()
                    .await
                    .map_err(ResolveError::OpenTokenStream)?;
                send.write_all(token.as_bytes())
                    .await
                    .map_err(ResolveError::WriteTokenStream)?;
                send.finish()
                    .await
                    .map_err(ResolveError::FinishTokenStream)?;
                spawn(recv::connection(bi_streams, uni_streams, datagrams, tx));
                Ok((
                    Connection::remote(send::Connection::new(connection)),
                    rx,
                    None,
                ))
            }
        }
    }
}
