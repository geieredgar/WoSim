use std::sync::Arc;

use actor::Address;
use bytes::Bytes;
use futures::{future::join_all, StreamExt};
use log::warn;
use quinn::{
    Connecting, Datagrams, Incoming, IncomingBiStreams, IncomingUniStreams, NewConnection,
    RecvStream, SendStream, VarInt,
};
use tokio::spawn;

use crate::{
    from_bi_stream, from_datagram, from_uni_stream, Authenticator, EstablishConnectionError,
    Message, Reader, RemoteSender, SessionMessage,
};

pub fn listen<M: Message, A: Authenticator>(
    mut incoming: Incoming,
    authenticator: Arc<A>,
    receiver: Address<SessionMessage<A::Identity, M>>,
) {
    spawn(async move {
        while let Some(connecting) = incoming.next().await {
            spawn(accept(connecting, authenticator.clone(), receiver.clone()));
        }
    });
}

async fn accept<M: Message, A: Authenticator>(
    connecting: Connecting,
    authenticator: Arc<A>,
    receiver: Address<SessionMessage<A::Identity, M>>,
) -> Result<(), EstablishConnectionError> {
    let NewConnection {
        connection,
        bi_streams,
        mut uni_streams,
        datagrams,
        ..
    } = connecting.await?;
    let recv = uni_streams
        .next()
        .await
        .ok_or(EstablishConnectionError::TokenMissing)??;
    let token = Reader::recv(recv, A::token_size_limit())
        .await?
        .read()
        .map_err(EstablishConnectionError::Deserialize)?;
    let client = Address::new(Arc::new(RemoteSender(connection.clone())));
    let identity = match authenticator.authenticate(client, token) {
        Ok(identity) => identity,
        Err(error) => {
            let reason = error.to_string();
            connection.close(VarInt::from_u32(1), reason.as_bytes());
            return Err(EstablishConnectionError::TokenRejected(reason));
        }
    };
    receiver.send(SessionMessage::Connect(identity.clone()));
    join_all(vec![
        spawn(self::bi_streams(
            bi_streams,
            receiver.clone(),
            identity.clone(),
        )),
        spawn(self::uni_streams(
            uni_streams,
            receiver.clone(),
            identity.clone(),
        )),
        spawn(self::datagrams(
            datagrams,
            receiver.clone(),
            identity.clone(),
        )),
    ])
    .await;
    receiver.send(SessionMessage::Disconnect(identity));
    Ok(())
}

async fn bi_streams<I: Clone + Send, M: Message>(
    mut bi_streams: IncomingBiStreams,
    receiver: Address<SessionMessage<I, M>>,
    identity: I,
) {
    while let Some(Ok((send, recv))) = bi_streams.next().await {
        spawn(bi_stream(send, recv, receiver.clone(), identity.clone()));
    }
}

async fn uni_streams<I: Clone + Send, M: Message>(
    mut uni_streams: IncomingUniStreams,
    receiver: Address<SessionMessage<I, M>>,
    identity: I,
) {
    while let Some(Ok(recv)) = uni_streams.next().await {
        spawn(uni_stream(recv, receiver.clone(), identity.clone()));
    }
}

async fn datagrams<I: Clone + Send, M: Message>(
    mut datagrams: Datagrams,
    receiver: Address<SessionMessage<I, M>>,
    identity: I,
) {
    while let Some(Ok(datagram)) = datagrams.next().await {
        spawn(self::datagram(datagram, receiver.clone(), identity.clone()));
    }
}

async fn bi_stream<I, M: Message>(
    send: SendStream,
    recv: RecvStream,
    receiver: Address<SessionMessage<I, M>>,
    identity: I,
) {
    let message = match from_bi_stream(recv, send).await {
        Ok(message) => message,
        Err(error) => {
            warn!("Reading bidirectional stream failed: {}", error);
            return;
        }
    };
    receiver.send(SessionMessage::Message(identity, message))
}

async fn uni_stream<I, M: Message>(
    recv: RecvStream,
    receiver: Address<SessionMessage<I, M>>,
    identity: I,
) {
    let message = match from_uni_stream(recv).await {
        Ok(message) => message,
        Err(error) => {
            warn!("Reading unidirectional stream failed: {}", error);
            return;
        }
    };
    receiver.send(SessionMessage::Message(identity, message))
}

async fn datagram<I, M: Message>(
    datagram: Bytes,
    receiver: Address<SessionMessage<I, M>>,
    identity: I,
) {
    let message = match from_datagram(datagram) {
        Ok(message) => message,
        Err(error) => {
            warn!("Reading datagram failed: {}", error);
            return;
        }
    };
    receiver.send(SessionMessage::Message(identity, message));
}
