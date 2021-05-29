use std::{
    io,
    net::{IpAddr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    str::FromStr,
    sync::Arc,
};

use quinn::{
    ClientConfigBuilder, ConnectError, ConnectionError, Endpoint, EndpointError, NewConnection,
    WriteError,
};
use tokio::{spawn, sync::mpsc};

use crate::{recv, send, AuthToken, Connection, Server, Service, Verification};

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
                Ok((
                    Connection::local(tx),
                    rx,
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
