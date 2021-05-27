use std::{io, net::SocketAddr, str::Utf8Error, sync::Arc};

use futures::StreamExt;

use libmdns::Responder;
use quinn::{
    crypto::rustls::TLSError, Connecting, ConnectionError, Endpoint, EndpointError, Incoming,
    NewConnection, ReadToEndError, ServerConfig, ServerConfigBuilder, VarInt,
};
use tokio::spawn;

use crate::{session, AuthToken, Connection, RemoteConnection, Service};

pub struct Listener {
    endpoint: Endpoint,
    _service: libmdns::Service,
}

#[derive(Debug)]
pub enum OpenError {
    TLS(TLSError),
    Endpoint(EndpointError),
    LocalAddress(io::Error),
}

enum AcceptError<S: Service> {
    Connecting(ConnectionError),
    MissingToken,
    OpenTokenStream(ConnectionError),
    ReadToken(ReadToEndError),
    InvalidToken(Utf8Error),
    TokenAuthentication(S::AuthError),
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
            .map_err(OpenError::TLS)?;
        let server_config = server_config.build();
        let mut endpoint = Endpoint::builder();
        endpoint.listen(server_config);
        let (endpoint, incoming) = endpoint.bind(address).map_err(OpenError::Endpoint)?;
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
            .close(VarInt::from_u32(2), "Server closed".as_bytes());
    }
}

async fn listen<S: Service>(mut incoming: Incoming, service: Arc<S>) {
    while let Some(connecting) = incoming.next().await {
        spawn(accept(connecting, service.clone()));
    }
}

async fn accept<S: Service>(connecting: Connecting, service: Arc<S>) -> Result<(), AcceptError<S>> {
    let NewConnection {
        connection,
        bi_streams,
        mut uni_streams,
        datagrams,
        ..
    } = connecting.await.map_err(AcceptError::Connecting)?;
    let recv = uni_streams
        .next()
        .await
        .ok_or(AcceptError::MissingToken)?
        .map_err(AcceptError::OpenTokenStream)?;
    let buffer = recv
        .read_to_end(service.token_size_limit())
        .await
        .map_err(AcceptError::ReadToken)?;
    let token = std::str::from_utf8(&buffer).map_err(AcceptError::InvalidToken)?;
    let connection = Connection::Remote(RemoteConnection::new(connection));
    let receiver = service
        .authenticate(connection, AuthToken::Remote(token))
        .map_err(AcceptError::TokenAuthentication)?;
    session(bi_streams, uni_streams, datagrams, receiver).await;
    Ok(())
}
