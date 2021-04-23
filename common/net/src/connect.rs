use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use actor::{forward, mailbox, Address};
use bytes::Bytes;
use futures::StreamExt;
use log::warn;
use quinn::{
    Datagrams, Endpoint, IncomingBiStreams, IncomingUniStreams, NewConnection, RecvStream,
    SendStream,
};
use serde::Serialize;
use tokio::spawn;

use crate::{
    from_bi_stream, from_datagram, from_uni_stream,
    sender::{LocalSender, RemoteSender},
    Authenticator, EstablishConnectionError, Message, SessionMessage, Writer,
};

pub fn local_connect<M, A: Authenticator, F: FnOnce(Address<M>) -> Address<A::ClientMessage>>(
    server: Address<SessionMessage<A::Identity, M>>,
    authenticator: &A,
    factory: F,
    token: A::Token,
) -> Result<(Address<A::ClientMessage>, Address<M>), EstablishConnectionError> {
    let (mailbox, client) = mailbox();
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
    F: FnOnce(Address<N>) -> Address<M>,
>(
    endpoint: &Endpoint,
    addr: &SocketAddr,
    server_name: &str,
    factory: F,
    token: &T,
) -> Result<(Address<M>, Address<N>), EstablishConnectionError> {
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
    spawn(self::bi_streams(bi_streams, client.clone()));
    spawn(self::uni_streams(uni_streams, client.clone()));
    spawn(self::datagrams(datagrams, client.clone()));
    Ok((client, server))
}

async fn bi_streams<M: Message>(mut bi_streams: IncomingBiStreams, receiver: Address<M>) {
    while let Some(Ok((send, recv))) = bi_streams.next().await {
        spawn(bi_stream(send, recv, receiver.clone()));
    }
}

async fn uni_streams<M: Message>(mut uni_streams: IncomingUniStreams, receiver: Address<M>) {
    while let Some(Ok(recv)) = uni_streams.next().await {
        spawn(uni_stream(recv, receiver.clone()));
    }
}

async fn datagrams<M: Message>(mut datagrams: Datagrams, receiver: Address<M>) {
    while let Some(Ok(datagram)) = datagrams.next().await {
        spawn(self::datagram(datagram, receiver.clone()));
    }
}

async fn bi_stream<M: Message>(send: SendStream, recv: RecvStream, receiver: Address<M>) {
    let message = match from_bi_stream(recv, send).await {
        Ok(message) => message,
        Err(error) => {
            warn!("Reading bidirectional stream failed: {}", error);
            return;
        }
    };
    receiver.send(message)
}

async fn uni_stream<M: Message>(recv: RecvStream, receiver: Address<M>) {
    let message = match from_uni_stream(recv).await {
        Ok(message) => message,
        Err(error) => {
            warn!("Reading unidirectional stream failed: {}", error);
            return;
        }
    };
    receiver.send(message)
}

async fn datagram<M: Message>(datagram: Bytes, receiver: Address<M>) {
    let message = match from_datagram(datagram) {
        Ok(message) => message,
        Err(error) => {
            warn!("Reading datagram failed: {}", error);
            return;
        }
    };
    receiver.send(message);
}
