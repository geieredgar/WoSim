use bytes::{Bytes, BytesMut};
use eyre::eyre;
use log::error;
use quinn::{RecvStream, SendStream};
use tokio::{spawn, sync::mpsc};

use crate::{IncomingMessage, Message};

use super::{
    error::{ConvertErr, Error},
    Return,
};

const CHANNEL_BUFFER: usize = 16;

pub async fn bi_stream<M: Message>(
    send: SendStream,
    mut recv: RecvStream,
    tx: mpsc::Sender<M>,
) -> Result<(), Error> {
    let mut buf = [0; 4];
    recv.read_exact(&mut buf)
        .await
        .convert_err("could not read message id")?;
    let id = u32::from_be_bytes(buf);
    if id == 0 {
        let mut key = 1;
        let (send_channel, recv_channel) = mpsc::channel(CHANNEL_BUFFER);
        spawn(async move {
            if let Err(Error::Error(error)) = send_responses(send, recv_channel).await {
                error!("{:?}", error);
            }
        });
        loop {
            let mut buf = [0; 4];
            recv.read_exact(&mut buf)
                .await
                .convert_err("could not read message id")?;
            let id = u32::from_be_bytes(buf);
            if id == 0 {
                break;
            }
            recv_request(
                &mut recv,
                &tx,
                id,
                Return::shared(key, send_channel.clone()),
            )
            .await?;
            key += 1;
        }
    } else {
        recv_request(&mut recv, &tx, id, Return::unique(send)).await?;
    }
    Ok(())
}

pub async fn uni_stream<M: Message>(
    mut recv: RecvStream,
    tx: mpsc::Sender<M>,
) -> Result<(), Error> {
    let mut buf = [0; 4];
    recv.read_exact(&mut buf)
        .await
        .convert_err("could not read message id")?;
    let id = u32::from_be_bytes(buf);
    recv_request(&mut recv, &tx, id, Return::none()).await
}

async fn send_responses(
    mut send: SendStream,
    mut receiver: mpsc::Receiver<(u32, Bytes)>,
) -> Result<(), Error> {
    while let Some((key, buf)) = receiver.recv().await {
        send.write_all(&key.to_be_bytes())
            .await
            .convert_err("could not write message key")?;
        send.write_all(&buf)
            .await
            .convert_err("could not write message data")?;
    }
    send.write_all(&0u32.to_be_bytes())
        .await
        .convert_err("could not write multi-message end marker")
}

async fn recv_request<M: Message>(
    recv: &mut RecvStream,
    tx: &mpsc::Sender<M>,
    id: u32,
    r#return: Return,
) -> Result<(), Error> {
    let size_limit = M::size_limit(id);
    let mut buf = [0; 4];
    recv.read_exact(&mut buf)
        .await
        .convert_err("could not read message size")?;
    let size = u32::from_be_bytes(buf) as usize;
    if size > size_limit {
        return Err(Error::Error(eyre!(
            "message is larger than allowed: {} > {}",
            size,
            size_limit
        )));
    }
    let mut buf = BytesMut::with_capacity(size);
    unsafe { buf.set_len(size) };
    recv.read_exact(&mut buf)
        .await
        .convert_err("could not read message data")?;
    let message = IncomingMessage::new(id, buf.freeze(), r#return);
    let message = M::from_incoming(message).convert_err("could not convert message")?;
    tx.send(message).await.convert_err("could not send message")
}
