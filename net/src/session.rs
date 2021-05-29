use bytes::{Buf, Bytes, BytesMut};
use futures::{future::join_all, StreamExt};
use log::error;
use quinn::{Datagrams, IncomingBiStreams, IncomingUniStreams, RecvStream, SendStream};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    spawn,
    sync::mpsc::{self, unbounded_channel, UnboundedReceiver},
};

use crate::{IncomingMessage, Message, RecvError, SendError, Sender};

pub(super) async fn session<M: Message>(
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
            if let Err(error) = bi_stream(send, recv, tx).await {
                error!("{}", error);
            }
        });
    }
}

async fn uni_streams<M: Message>(mut uni_streams: IncomingUniStreams, tx: mpsc::Sender<M>) {
    while let Some(Ok(recv)) = uni_streams.next().await {
        let tx = tx.clone();
        spawn(async move {
            if let Err(error) = uni_stream(recv, tx).await {
                error!("{}", error);
            }
        });
    }
}

async fn datagrams<M: Message>(mut datagrams: Datagrams, tx: mpsc::Sender<M>) {
    while let Some(Ok(datagram)) = datagrams.next().await {
        let tx = tx.clone();
        spawn(async move {
            if let Err(error) = self::datagram(datagram, tx).await {
                error!("{}", error);
            }
        });
    }
}

async fn bi_stream<M: Message>(
    send: SendStream,
    mut recv: RecvStream,
    tx: mpsc::Sender<M>,
) -> Result<(), RecvError> {
    let id = recv.read_u32().await.map_err(RecvError::ReadRequestId)?;
    if id == 0 {
        let mut count = 1;
        let (send_channel, recv_channel) = unbounded_channel();
        spawn(async move {
            if let Err(error) = send_responses(send, recv_channel).await {
                error!("{}", error);
            }
        });
        loop {
            let id = recv.read_u32().await.map_err(RecvError::ReadRequestId)?;
            if id == 0 {
                break;
            }
            recv_request(
                &mut recv,
                &tx,
                id,
                Sender::Shared(count, send_channel.clone()),
            )
            .await?;
            count += 1;
        }
    } else {
        recv_request(&mut recv, &tx, id, Sender::Unique(send)).await?;
    }
    Ok(())
}

async fn uni_stream<M: Message>(
    mut recv: RecvStream,
    tx: mpsc::Sender<M>,
) -> Result<(), RecvError> {
    let id = recv.read_u32().await.map_err(RecvError::ReadRequestId)?;
    recv_request(&mut recv, &tx, id, Sender::None).await
}

async fn send_responses(
    mut send: SendStream,
    mut receiver: UnboundedReceiver<(u32, Bytes)>,
) -> Result<(), SendError> {
    while let Some((id, buf)) = receiver.recv().await {
        send.write_u32(id)
            .await
            .map_err(SendError::WriteResponseKey)?;
        send.write_all(&buf)
            .await
            .map_err(SendError::WriteResponseData)?;
    }
    send.write_u32(0)
        .await
        .map_err(SendError::WriteResponseKey)?;
    Ok(())
}

async fn recv_request<M: Message>(
    recv: &mut RecvStream,
    tx: &mpsc::Sender<M>,
    id: u32,
    sender: Sender,
) -> Result<(), RecvError> {
    let size_limit = M::size_limit(id);
    let size = recv.read_u32().await.map_err(RecvError::ReadRequestSize)? as usize;
    if size > size_limit {
        return Err(RecvError::RequestTooLarge { size, size_limit });
    }
    let mut buf = BytesMut::with_capacity(size);
    unsafe { buf.set_len(size) };
    recv.read_exact(&mut buf)
        .await
        .map_err(RecvError::ReadRequestData)?;
    let message = IncomingMessage::new(id, buf.freeze(), sender);
    let message = M::from_incoming(message).map_err(RecvError::FromIncoming)?;
    let _ = tx.send(message).await;
    Ok(())
}

async fn datagram<M: Message>(mut datagram: Bytes, tx: mpsc::Sender<M>) -> Result<(), RecvError> {
    let id = datagram.get_u32();
    let message = IncomingMessage::new(id, datagram, Sender::None);
    let message = M::from_incoming(message).map_err(RecvError::FromIncoming)?;
    let _ = tx.send(message).await;
    Ok(())
}
