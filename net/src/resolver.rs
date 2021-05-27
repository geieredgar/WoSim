use std::{
    io,
    net::{IpAddr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    str::FromStr,
    sync::Arc,
};

use actor::{mailbox, Mailbox};
use quinn::{
    ClientConfigBuilder, ConnectError, ConnectionError, Endpoint, EndpointError, NewConnection,
    WriteError,
};
use tokio::spawn;

use crate::{session, AuthToken, Connection, RemoteConnection, Server, Service, Verification};

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
    Mailbox<<S as Service>::Push>,
    Option<Server<S>>,
);
#[derive(Debug)]
pub enum ResolveError<A> {
    TokenAuthentication(A),
    InvalidCaCertificates(webpki::Error),
    Bind(EndpointError),
    IpResolution(io::Error),
    NoSocketAddrFound,
    Connect(ConnectError),
    Connecting(ConnectionError),
    OpenTokenStream(ConnectionError),
    WriteTokenStream(WriteError),
    FinishTokenStream(WriteError),
}

impl<S: Service> Resolver<S> {
    pub async fn resolve(self) -> ResolveResult<S> {
        match self {
            Resolver::Local {
                service,
                token,
                port,
            } => {
                let (mailbox, address) = mailbox();
                let address = service
                    .authenticate(Connection::Local(address), AuthToken::Local(&token))
                    .map_err(ResolveError::TokenAuthentication)?;
                Ok((
                    Connection::Local(address),
                    mailbox,
                    Some(Server::new(
                        service,
                        SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port),
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
                let (endpoint, _) = endpoint
                    .bind(&"[::]:0".parse().unwrap())
                    .map_err(ResolveError::Bind)?;
                let address = (hostname.as_str(), port)
                    .to_socket_addrs()
                    .map_err(ResolveError::IpResolution)?
                    .next()
                    .ok_or(ResolveError::NoSocketAddrFound)?;
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
                    .connect(&address, server_name)
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
                let (mailbox, client) = mailbox();
                spawn(session(bi_streams, uni_streams, datagrams, client));
                Ok((
                    Connection::Remote(RemoteConnection::new(connection)),
                    mailbox,
                    None,
                ))
            }
        }
    }
}
