use std::{net::SocketAddr, sync::Arc};

use actor::Address;
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

pub fn local_connect<M, A: Authenticator>(
    server: Address<SessionMessage<A::Identity, M>>,
    authenticator: &A,
    client: Address<A::ClientMessage>,
    token: A::Token,
) -> Result<Address<M>, EstablishConnectionError> {
    match authenticator.authenticate(client, token) {
        Ok(identity) => Ok(Address::new(Arc::new(LocalSender::new(server, identity)))),
        Err(error) => Err(EstablishConnectionError::TokenRejected(error.to_string())),
    }
}

pub async fn remote_connect<M: Message, N: Message, T: Serialize>(
    endpoint: &Endpoint,
    addr: &SocketAddr,
    server_name: &str,
    receiver: Address<M>,
    token: &T,
) -> Result<Address<N>, EstablishConnectionError> {
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
    spawn(self::bi_streams(bi_streams, receiver.clone()));
    spawn(self::uni_streams(uni_streams, receiver.clone()));
    spawn(self::datagrams(datagrams, receiver.clone()));
    Ok(Address::new(Arc::new(RemoteSender(connection))))
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
