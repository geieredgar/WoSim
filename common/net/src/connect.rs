use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use actor::{forward, mailbox, Address};
use quinn::{Endpoint, NewConnection};
use serde::Serialize;
use tokio::spawn;

use crate::{
    sender::{LocalSender, RemoteSender},
    session, Authenticator, EstablishConnectionError, Message, SessionMessage, Writer,
};

type ConnectResult<T> = Result<T, EstablishConnectionError>;

type LocalConnectResult<I, M, N> = ConnectResult<(Address<SessionMessage<I, M>>, Address<N>)>;

pub fn local_connect<
    M,
    A: Authenticator,
    I: Clone + Send + Sync + 'static,
    F: FnOnce(Address<M>) -> Address<SessionMessage<I, A::ClientMessage>>,
>(
    server: Address<SessionMessage<A::Identity, M>>,
    authenticator: &A,
    factory: F,
    identity: I,
    token: A::Token,
) -> LocalConnectResult<I, A::ClientMessage, M> {
    let (mailbox, client) = mailbox();
    let client = Address::new(Arc::new(LocalSender::new(client, identity)));
    let identity = match authenticator.authenticate(client, token) {
        Ok(identity) => identity,
        Err(error) => return Err(EstablishConnectionError::TokenRejected(error.to_string())),
    };
    let server = Address::new(Arc::new(LocalSender::new(server, identity)));
    let client = factory(server.clone());
    spawn(forward(mailbox, client.clone()));
    Ok((client, server))
}

pub async fn remote_connect<
    M: Message,
    N: Message,
    T: Serialize,
    I: Clone + 'static + Send + Sync,
    F: FnOnce(Address<N>) -> Address<SessionMessage<I, M>>,
>(
    endpoint: &Endpoint,
    addr: &SocketAddr,
    server_name: &str,
    factory: F,
    identity: I,
    token: &T,
) -> ConnectResult<(Address<SessionMessage<I, M>>, Address<N>)> {
    let server_name = if IpAddr::from_str(server_name).is_err() {
        server_name
    } else {
        "localhost"
    };
    let NewConnection {
        connection,
        bi_streams,
        uni_streams,
        datagrams,
        ..
    } = endpoint.connect(addr, server_name)?.await?;
    let send = connection.open_uni().await?;
    let mut writer = Writer::new();
    writer
        .write(token)
        .map_err(EstablishConnectionError::Serialize)?;
    writer.send(send).await?;
    let server = Address::new(Arc::new(RemoteSender(connection)));
    let client = factory(server.clone());
    spawn(session(
        bi_streams,
        uni_streams,
        datagrams,
        client.clone(),
        identity,
    ));
    Ok((client, server))
}
