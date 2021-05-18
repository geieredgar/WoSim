use std::{
    fmt::Debug,
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use actor::{mailbox, Address, Mailbox};
use quinn::{Connection, Endpoint, NewConnection};
use serde::Serialize;
use tokio::spawn;

use crate::{
    sender::RemoteSender, session, Authenticator, EstablishConnectionError, Message, Writer,
};

type ConnectResult<T> = Result<T, EstablishConnectionError>;

type LocalConnectResult<M, N> = ConnectResult<(Address<M>, Address<N>)>;

pub fn local_connect<
    A: Authenticator,
    F: FnOnce(Address<A::ServerMessage>, Mailbox<A::ClientMessage>),
>(
    authenticator: &A,
    factory: F,
    token: A::Token,
) -> LocalConnectResult<A::ClientMessage, A::ServerMessage> {
    let (mailbox, client) = mailbox();
    let server = match authenticator.authenticate(client.clone(), token) {
        Ok(server) => server,
        Err(error) => return Err(EstablishConnectionError::TokenRejected(error.to_string())),
    };
    factory(server.clone(), mailbox);
    Ok((client, server))
}

pub async fn remote_connect<
    M: Message + Debug,
    N: Message + Debug,
    T: Serialize,
    F: FnOnce(Address<N>, Mailbox<M>),
>(
    endpoint: &Endpoint,
    addr: &SocketAddr,
    server_name: &str,
    factory: F,
    token: &T,
) -> ConnectResult<(Address<M>, Address<N>, Connection)> {
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
    let sender = RemoteSender(connection.clone());
    let server = Address::new(move |message| {
        sender.send(message);
        Ok(())
    });
    let (mailbox, client) = mailbox();
    spawn(session(bi_streams, uni_streams, datagrams, client.clone()));
    factory(server.clone(), mailbox);
    Ok((client, server, connection))
}
