use std::sync::{Arc, Weak};

use actor::Address;
use log::{error, warn};
use quinn::{ReadToEndError, RecvStream, SendDatagramError, SendStream, VarInt};
use serde::Serialize;
use tokio::spawn;

use crate::{Host, NetAddress, Protocol, Reader, RemoteSender, ReturnAddress, Writer};

#[derive(Clone)]
pub struct Connection(Arc<ConnectionInner>);

impl Connection {
    pub(super) fn new(
        connection: quinn::Connection,
        id: u64,
        size_limit: usize,
        host: Host,
    ) -> Self {
        Self(Arc::new(ConnectionInner {
            connection,
            id,
            size_limit,
            host,
        }))
    }

    pub(super) fn downgrade(&self) -> WeakConnection {
        WeakConnection(Arc::downgrade(&self.0))
    }

    pub fn address<T: Protocol>(&self, port: u16) -> NetAddress<T> {
        NetAddress::new(
            Address::new(Arc::new(RemoteSender::new(self.clone(), port))),
            None,
        )
    }

    pub fn return_address<T: Serialize + Send + 'static>(
        &self,
        send: SendStream,
    ) -> ReturnAddress<T> {
        ReturnAddress::Remote(send, self.clone())
    }

    pub fn id(&self) -> u64 {
        self.0.id
    }

    pub fn host(&self) -> &Host {
        &self.0.host
    }

    pub fn send_uni(self, writer: Writer) {
        spawn(self.handle_send_uni(writer.into_inner()));
    }

    async fn handle_send_uni(self, bytes: Vec<u8>) {
        let mut send = match self.0.connection.open_uni().await {
            Ok(send) => send,
            Err(error) => {
                warn!("Open unidirectional stream failed: {}", error);
                return;
            }
        };
        if let Err(error) = send.write_all(&bytes).await {
            warn!("Writing message to unidirectional stream failed: {}", error);
            return;
        };
        if let Err(error) = send.finish().await {
            warn!("Shutting down unidirectional stream failed: {}", error)
        }
    }

    pub fn send_datagram(&self, writer: Writer) -> Result<(), SendDatagramError> {
        self.0.connection.send_datagram(writer.into_inner().into())
    }

    pub fn send_bi<F: FnOnce(Reader) + 'static + Send, E>(self, writer: Writer, callback: F) {
        spawn(self.handle_send_bi(writer.into_inner(), callback));
    }

    async fn handle_send_bi<F: FnOnce(Reader)>(self, bytes: Vec<u8>, callback: F) {
        let (mut send, recv) = match self.0.connection.open_bi().await {
            Ok(streams) => streams,
            Err(error) => {
                warn!("Open bidirectional stream failed: {}", error);
                return;
            }
        };
        if let Err(error) = send.write_all(&bytes).await {
            warn!("Writing message to bidirectional stream failed: {}", error);
            return;
        };
        if let Err(error) = send.finish().await {
            warn!("Shutting down send stream failed: {}", error)
        }
        let bytes = match recv.read_to_end(self.0.size_limit).await {
            Ok(bytes) => bytes,
            Err(error) => {
                error!(
                    "Reading response from bidirectional stream failed: {}",
                    error
                );
                return;
            }
        };
        callback(Reader::new(bytes.into()))
    }

    pub(super) async fn read(&self, recv: RecvStream) -> Result<Reader, ReadToEndError> {
        Ok(Reader::new(
            recv.read_to_end(self.0.size_limit).await?.into(),
        ))
    }
}

pub(super) struct WeakConnection(Weak<ConnectionInner>);

impl WeakConnection {
    pub(super) fn upgrade(&self) -> Option<Connection> {
        self.0.upgrade().map(Connection)
    }
}

struct ConnectionInner {
    connection: quinn::Connection,
    id: u64,
    size_limit: usize,
    host: Host,
}

impl Drop for ConnectionInner {
    fn drop(&mut self) {
        self.connection.close(VarInt::from_u32(0), &[]);
    }
}
