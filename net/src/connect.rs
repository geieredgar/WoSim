use std::{
    fmt::Debug,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use actor::{mailbox, Address, Mailbox};
use quinn::{Endpoint, NewConnection};
use tokio::spawn;

use crate::{sender::RemoteSender, session, Client, EstablishConnectionError, Server};

#[derive(Debug)]
pub enum Connection<S: Server> {
    Local(Arc<S>),
    Remote(quinn::Connection),
}

pub type ConnectResult<S> = Result<
    (
        Address<<S as Server>::Request>,
        Mailbox<<S as Server>::Push>,
        Connection<S>,
    ),
    EstablishConnectionError,
>;

impl<S: Server> Connection<S> {
    pub fn rtt(&self) -> Duration {
        match self {
            Connection::Local(_) => Duration::from_secs(0),
            Connection::Remote(connection) => connection.rtt(),
        }
    }
}

pub fn local_connect<S: Server>(server: Arc<S>) -> ConnectResult<S> {
    let (mailbox, address) = mailbox();
    let address = match server.authenticate(Client::Local, address) {
        Ok(address) => address,
        Err(error) => return Err(EstablishConnectionError::TokenRejected(error.to_string())),
    };
    Ok((address, mailbox, Connection::Local(server)))
}

pub async fn remote_connect<S: Server>(
    endpoint: &Endpoint,
    addr: &SocketAddr,
    server_name: &str,
    token: &str,
) -> ConnectResult<S> {
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
    let mut send = connection.open_uni().await?;
    send.write_all(token.as_bytes()).await?;
    send.finish().await?;
    let sender = RemoteSender(connection.clone());
    let server = Address::new(move |message| {
        sender.send(message);
        Ok(())
    });
    let (mailbox, client) = mailbox();
    spawn(session(bi_streams, uni_streams, datagrams, client));
    Ok((server, mailbox, Connection::Remote(connection)))
}
