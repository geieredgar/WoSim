use std::{error::Error as StdError, fmt::Display, sync::Arc};

use bytes::{Buf, Bytes, BytesMut};
use log::error;
use quinn::{RecvStream, SendStream, VarInt};
use tokio::{
    io::AsyncReadExt,
    io::AsyncWriteExt,
    spawn,
    sync::{mpsc, oneshot::Sender},
};

pub use quinn_proto::ConnectionStats;

use crate::{Message, RecvError, RecvQueue, SendError};

const CHANNEL_BUFFER: usize = 16;

#[derive(Debug)]
pub enum Connection<M: Message> {
    Local(mpsc::Sender<M>),
    Remote(RemoteConnection<M>),
}

#[derive(Debug)]
pub struct RemoteConnection<M: Message> {
    tx: mpsc::Sender<M>,
    inner: quinn::Connection,
    closer: Arc<AutoCloser>,
}

#[derive(Debug)]
struct AutoCloser(quinn::Connection);

impl<M: Message> Connection<M> {
    pub fn asynchronous(&self) -> mpsc::Sender<M> {
        match self {
            Connection::Local(tx) => tx.clone(),
            Connection::Remote(connection) => connection.tx.clone(),
        }
    }

    pub fn synchronous(&self) -> mpsc::Sender<M> {
        match self {
            Connection::Local(tx) => tx.clone(),
            Connection::Remote(connection) => {
                let (send, recv) = mpsc::channel(CHANNEL_BUFFER);
                let connection = connection.clone();
                spawn(async move {
                    if let Err(error) = send_messages(connection, recv).await {
                        error!("{}", error)
                    }
                });
                send
            }
        }
    }

    pub fn stats(&self) -> Option<ConnectionStats> {
        match self {
            Connection::Local(_) => None,
            Connection::Remote(connection) => Some(connection.inner.stats()),
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

impl<M: Message> RemoteConnection<M> {
    pub fn new(connection: quinn::Connection) -> Self {
        let closer = Arc::new(AutoCloser(connection.clone()));
        let (tx, mut rx) = mpsc::channel(CHANNEL_BUFFER);
        {
            let connection = connection.clone();
            spawn(async move {
                while let Some(message) = rx.recv().await {
                    let connection = connection.clone();
                    spawn(async move {
                        if let Err(error) = send_message(connection, message).await {
                            error!("{}", error)
                        }
                    });
                }
            });
        }
        Self {
            tx,
            inner: connection,
            closer,
        }
    }
}

impl<M: Message> Clone for RemoteConnection<M> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            closer: self.closer.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl Drop for AutoCloser {
    fn drop(&mut self) {
        self.0.close(VarInt::from_u32(0), &[]);
    }
}

#[derive(Default)]
pub struct ConnectionStatsDiff {
    pub tx: UdpStatsDiff,
    pub rx: UdpStatsDiff,
}

impl ConnectionStatsDiff {
    pub fn new(from: ConnectionStats, to: ConnectionStats) -> Self {
        Self {
            tx: UdpStatsDiff {
                datagrams: to.udp_tx.datagrams - from.udp_tx.datagrams,
                bytes: to.udp_tx.bytes - from.udp_tx.bytes,
                transmits: to.udp_tx.transmits - from.udp_tx.transmits,
            },
            rx: UdpStatsDiff {
                datagrams: to.udp_rx.datagrams - from.udp_rx.datagrams,
                bytes: to.udp_rx.bytes - from.udp_rx.bytes,
                transmits: to.udp_rx.transmits - from.udp_rx.transmits,
            },
        }
    }
}

#[derive(Default)]
pub struct UdpStatsDiff {
    pub datagrams: u64,
    pub bytes: u64,
    pub transmits: u64,
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

impl StdError for Error {}

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
    connection: RemoteConnection<M>,
    mut recv: mpsc::Receiver<M>,
) -> Result<(), SendError> {
    let queue = RecvQueue::new();
    let mut send = {
        let (send, recv) = connection
            .inner
            .open_bi()
            .await
            .map_err(SendError::OpenStream)?;
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
