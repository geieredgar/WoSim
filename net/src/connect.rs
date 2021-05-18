use std::{
    fmt::Debug,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Duration,
};

use actor::{mailbox, Address, Mailbox};
use quinn::{Endpoint, NewConnection};
use serde::Serialize;
use tokio::spawn;

use crate::{sender::RemoteSender, session, EstablishConnectionError, Message, Server, Writer};

#[derive(Debug)]
pub enum Connection {
    Local,
    Remote(quinn::Connection),
}

pub type ConnectResult<R, P> =
    Result<(Address<R>, Mailbox<P>, Connection), EstablishConnectionError>;

impl Connection {
    pub fn rtt(&self) -> Duration {
        match self {
            Connection::Local => Duration::from_secs(0),
            Connection::Remote(connection) => connection.rtt(),
        }
    }
}

pub fn local_connect<S: Server>(
    server: &S,
    token: S::AuthToken,
) -> ConnectResult<S::Request, S::Push> {
    let (mailbox, client) = mailbox();
    let server = match server.authenticate(client, token) {
        Ok(server) => server,
        Err(error) => return Err(EstablishConnectionError::TokenRejected(error.to_string())),
    };
    Ok((server, mailbox, Connection::Local))
}

pub async fn remote_connect<R: Message + Debug, P: Message + Debug, T: Serialize>(
    endpoint: &Endpoint,
    addr: &SocketAddr,
    server_name: &str,
    token: &T,
) -> ConnectResult<R, P> {
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
    spawn(session(bi_streams, uni_streams, datagrams, client));
    Ok((server, mailbox, Connection::Remote(connection)))
}
