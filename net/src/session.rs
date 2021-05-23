use actor::Address;
use bytes::Bytes;
use futures::{future::join_all, StreamExt};
use log::warn;
use quinn::{Datagrams, IncomingBiStreams, IncomingUniStreams, RecvStream, SendStream};
use tokio::spawn;

use crate::{from_bi_stream, from_datagram, from_uni_stream, Message};

pub(super) async fn session<M: Message>(
    bi_streams: IncomingBiStreams,
    uni_streams: IncomingUniStreams,
    datagrams: Datagrams,
    receiver: Address<M>,
) {
    join_all(vec![
        spawn(self::bi_streams(bi_streams, receiver.clone())),
        spawn(self::uni_streams(uni_streams, receiver.clone())),
        spawn(self::datagrams(datagrams, receiver.clone())),
    ])
    .await;
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
    let _ = receiver.send(message);
}

async fn uni_stream<M: Message>(recv: RecvStream, receiver: Address<M>) {
    let message = match from_uni_stream(recv).await {
        Ok(message) => message,
        Err(error) => {
            warn!("Reading unidirectional stream failed: {}", error);
            return;
        }
    };
    let _ = receiver.send(message);
}

async fn datagram<M: Message>(datagram: Bytes, receiver: Address<M>) {
    let message = match from_datagram(datagram) {
        Ok(message) => message,
        Err(error) => {
            warn!("Reading datagram failed: {}", error);
            return;
        }
    };
    let _ = receiver.send(message);
}
