use bytes::{Buf, Bytes};
use futures::{future::join_all, StreamExt};
use log::error;
use quinn::{Datagrams, IncomingBiStreams, IncomingUniStreams};
use tokio::{spawn, sync::mpsc};

use crate::{IncomingMessage, Message};

use super::{
    error::{ConvertErr, Error},
    stream::{bi_stream, uni_stream},
    Return,
};

pub async fn connection<M: Message>(
    bi_streams: IncomingBiStreams,
    uni_streams: IncomingUniStreams,
    datagrams: Datagrams,
    tx: mpsc::Sender<M>,
) {
    join_all(vec![
        spawn(self::bi_streams(bi_streams, tx.clone())),
        spawn(self::uni_streams(uni_streams, tx.clone())),
        spawn(self::datagrams(datagrams, tx.clone())),
    ])
    .await;
}

async fn bi_streams<M: Message>(mut bi_streams: IncomingBiStreams, tx: mpsc::Sender<M>) {
    while let Some(Ok((send, recv))) = bi_streams.next().await {
        let tx = tx.clone();
        spawn(async move {
            if let Err(Error::Error(error)) = bi_stream(send, recv, tx).await {
                error!("{:?}", error);
            }
        });
    }
}

async fn uni_streams<M: Message>(mut uni_streams: IncomingUniStreams, tx: mpsc::Sender<M>) {
    while let Some(Ok(recv)) = uni_streams.next().await {
        let tx = tx.clone();
        spawn(async move {
            if let Err(Error::Error(error)) = uni_stream(recv, tx).await {
                error!("{:?}", error);
            }
        });
    }
}

async fn datagrams<M: Message>(mut datagrams: Datagrams, tx: mpsc::Sender<M>) {
    while let Some(Ok(datagram)) = datagrams.next().await {
        let tx = tx.clone();
        spawn(async move {
            if let Err(Error::Error(error)) = self::datagram(datagram, tx).await {
                error!("{:?}", error);
            }
        });
    }
}

async fn datagram<M: Message>(mut datagram: Bytes, tx: mpsc::Sender<M>) -> Result<(), Error> {
    let id = datagram.get_u32();
    let message = IncomingMessage::new(id, datagram, Return::none());
    let message = M::from_incoming(message).convert_err("could not convert message")?;
    tx.send(message).await.convert_err("could not send message")
}
