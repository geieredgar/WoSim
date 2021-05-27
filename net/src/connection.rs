use std::{fmt::Display, time::Duration};

use actor::Address;
use bytes::{Buf, Bytes, BytesMut};
use log::error;
use quinn::{RecvStream, SendStream};
use tokio::{
    io::AsyncReadExt,
    io::AsyncWriteExt,
    spawn,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver},
        oneshot::Sender,
    },
};

use crate::{Message, RecvError, RecvQueue, SendError};

#[derive(Debug)]
pub enum Connection<M: Message + 'static> {
    Local(Address<M>),
    Remote(quinn::Connection),
}

impl<M: Message> Connection<M> {
    pub fn parallel(&self) -> Address<M> {
        match self {
            Connection::Local(address) => address.clone(),
            Connection::Remote(connection) => {
                let connection = connection.clone();
                Address::new(move |message: M| {
                    let connection = connection.clone();
                    spawn(async move {
                        if let Err(error) = send_message(connection, message).await {
                            error!("{}", error);
                        }
                    });
                    Ok(())
                })
            }
        }
    }

    pub fn sequential(&self) -> Address<M> {
        match self {
            Connection::Local(address) => address.clone(),
            Connection::Remote(connection) => {
                let (send, recv) = unbounded_channel();
                let connection = connection.clone();
                spawn(async move {
                    if let Err(error) = send_messages(connection, recv).await {
                        error!("{}", error)
                    }
                });
                Address::new(move |message: M| match send.send(message) {
                    Ok(_) => Ok(()),
                    Err(error) => Err(Box::new(error)),
                })
            }
        }
    }

    pub fn rtt(&self) -> Duration {
        match self {
            Connection::Local(_) => Duration::from_secs(0),
            Connection::Remote(connection) => connection.rtt(),
        }
    }
}

impl<M: Message + 'static> Clone for Connection<M> {
    fn clone(&self) -> Self {
        match self {
            Self::Local(address) => Self::Local(address.clone()),
            Self::Remote(connection) => Self::Remote(connection.clone()),
        }
    }
}

#[derive(Debug)]
enum Error {
    Send(SendError),
    Recv(RecvError),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<SendError> for Error {
    fn from(error: SendError) -> Self {
        Self::Send(error)
    }
}

impl From<RecvError> for Error {
    fn from(error: RecvError) -> Self {
        Self::Recv(error)
    }
}

async fn send_message<M: Message>(connection: quinn::Connection, message: M) -> Result<(), Error> {
    let message = message.into_outgoing().map_err(SendError::IntoOutgoing)?;
    match message.ty {
        crate::MessageType::Datagram => connection
            .send_datagram(message.packet)
            .map_err(SendError::SendDatagram)?,
        crate::MessageType::Uni => {
            let send = connection.open_uni().await.map_err(SendError::OpenStream)?;
            send_request_and_finish(send, message.packet).await?
        }
        crate::MessageType::Bi(sender, size_limit) => {
            let (send, mut recv) = connection.open_bi().await.map_err(SendError::OpenStream)?;
            send_request_and_finish(send, message.packet).await?;
            recv_response(&mut recv, sender, size_limit).await?;
        }
    }
    Ok(())
}

async fn send_request(send: &mut SendStream, mut packet: Bytes) -> Result<(), SendError> {
    send.write_u32(packet.get_u32())
        .await
        .map_err(SendError::WriteRequestId)?;
    send.write_u32(packet.remaining() as u32)
        .await
        .map_err(SendError::WriteRequestSize)?;
    send.write_all(&packet)
        .await
        .map_err(SendError::WriteRequestData)
}

async fn send_request_and_finish(mut send: SendStream, packet: Bytes) -> Result<(), SendError> {
    send_request(&mut send, packet).await?;
    send.finish().await.map_err(SendError::FinishRequest)
}

async fn recv_response(
    recv: &mut RecvStream,
    sender: Sender<Bytes>,
    size_limit: usize,
) -> Result<(), RecvError> {
    let size = recv.read_u32().await.map_err(RecvError::ReadResponseSize)? as usize;
    if size > size_limit {
        return Err(RecvError::ResponseTooLarge { size, size_limit });
    }
    let mut buf = BytesMut::with_capacity(size);
    unsafe { buf.set_len(size) };
    recv.read_exact(&mut buf)
        .await
        .map_err(RecvError::ReadResponseData)?;
    sender
        .send(buf.freeze())
        .map_err(RecvError::SendResponseBytes)
}

async fn send_messages<M: Message>(
    connection: quinn::Connection,
    mut recv: UnboundedReceiver<M>,
) -> Result<(), SendError> {
    let queue = RecvQueue::new();
    let mut send = {
        let (send, recv) = connection.open_bi().await.map_err(SendError::OpenStream)?;
        let queue = queue.clone();
        spawn(async move {
            if let Err(error) = recv_responses(recv, queue).await {
                error!("{}", error)
            }
        });
        send
    };
    send.write_u32(0).await.map_err(SendError::WriteRequestId)?;
    let mut count = 1;
    while let Some(message) = recv.recv().await {
        let message = message.into_outgoing().map_err(SendError::IntoOutgoing)?;
        match message.ty {
            crate::MessageType::Datagram => send_request(&mut send, message.packet).await?,
            crate::MessageType::Uni => send_request(&mut send, message.packet).await?,
            crate::MessageType::Bi(sender, size_limit) => {
                queue.enqueue(count, sender, size_limit);
                send_request(&mut send, message.packet).await?
            }
        }
        count += 1;
    }
    send.write_u32(0).await.map_err(SendError::WriteRequestId)?;
    send.finish().await.map_err(SendError::FinishRequest)
}

async fn recv_responses(mut recv: RecvStream, queue: RecvQueue) -> Result<(), RecvError> {
    loop {
        let key = recv.read_u32().await.map_err(RecvError::ReadResponseKey)?;
        if key == 0 {
            return Ok(());
        }
        if let Some((sender, size_limit)) = queue.dequeue(key) {
            recv_response(&mut recv, sender, size_limit).await?;
        } else {
            return Err(RecvError::InvalidResponseKey(key));
        }
    }
}
