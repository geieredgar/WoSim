use std::{
    collections::{hash_map::Entry, HashMap},
    net::SocketAddr,
    ptr,
    sync::{Arc, RwLock},
};

use actor::Address;
use bytes::Bytes;
use futures::{future::join_all, StreamExt};
use log::warn;
use quinn::{
    Connecting, Datagrams, Endpoint, Incoming, IncomingBiStreams, IncomingUniStreams,
    NewConnection, RecvStream, SendStream,
};
use tokio::spawn;

use crate::{
    Authenticator, Connection, EstablishConnectionError, NetAddress, Protocol, Reader, Receiver,
    WeakConnection,
};

#[derive(Clone, Default)]
pub struct Host(Arc<HostInner>);

impl Host {
    pub fn bind<T: Protocol>(&self, address: Address<T>, port: u16) -> Option<NetAddress<T>> {
        let mut ports = self.0.ports.write().unwrap();
        if let Entry::Vacant(entry) = ports.entry(port) {
            entry.insert(Box::new(address.clone()));
            Some(NetAddress::new(
                address,
                Some(HostBinding {
                    host: self.clone(),
                    port,
                }),
            ))
        } else {
            None
        }
    }

    pub fn is_same(&self, other: &Host) -> bool {
        ptr::eq(&*self.0, &*other.0)
    }

    pub fn listen(
        &self,
        incoming: Incoming,
        authenticator: Arc<dyn Authenticator>,
        size_limit: usize,
    ) {
        spawn(
            self.clone()
                .handle_listen(incoming, authenticator, size_limit),
        );
    }

    pub async fn handle_listen(
        self,
        mut incoming: Incoming,
        authenticator: Arc<dyn Authenticator>,
        size_limit: usize,
    ) {
        while let Some(connecting) = incoming.next().await {
            spawn(
                self.clone()
                    .accept(connecting, authenticator.clone(), size_limit),
            );
        }
    }

    pub async fn connect(
        &self,
        endpoint: &Endpoint,
        addr: &SocketAddr,
        server_name: &str,
        token: &[u8],
        size_limit: usize,
    ) -> Result<Connection, EstablishConnectionError> {
        let NewConnection {
            connection,
            bi_streams,
            uni_streams,
            datagrams,
            ..
        } = endpoint.connect(addr, server_name)?.await?;
        let (mut send, recv) = connection.open_bi().await?;
        send.write_all(token).await?;
        send.finish().await?;
        let bytes = recv.read_to_end(size_limit).await?;
        let result: Result<u64, String> =
            bincode::deserialize(&bytes).map_err(EstablishConnectionError::Deserialize)?;
        let id = result.map_err(EstablishConnectionError::TokenRejected)?;
        let connection = Connection::new(connection, id, size_limit, self.clone());
        spawn(
            self.clone()
                .handle_bi_streams(bi_streams, connection.downgrade()),
        );
        spawn(
            self.clone()
                .handle_uni_streams(uni_streams, connection.downgrade()),
        );
        spawn(
            self.clone()
                .handle_datagrams(datagrams, connection.downgrade()),
        );
        Ok(connection)
    }

    async fn accept(
        self,
        connecting: Connecting,
        authenticator: Arc<dyn Authenticator>,
        size_limit: usize,
    ) -> Result<(), EstablishConnectionError> {
        let NewConnection {
            connection,
            mut bi_streams,
            uni_streams,
            datagrams,
            ..
        } = connecting.await?;
        let (mut send, recv) = bi_streams
            .next()
            .await
            .ok_or(EstablishConnectionError::TokenMissing)??;
        let token = recv.read_to_end(size_limit).await?;
        let result = authenticator.login(token);
        let bytes = bincode::serialize(&result).map_err(EstablishConnectionError::Serialize)?;
        send.write_all(&bytes).await?;
        send.finish().await?;
        let id = result.map_err(EstablishConnectionError::TokenRejected)?;
        let connection = Connection::new(connection, id, size_limit, self.clone());
        join_all(vec![
            spawn(
                self.clone()
                    .handle_bi_streams(bi_streams, connection.downgrade()),
            ),
            spawn(
                self.clone()
                    .handle_uni_streams(uni_streams, connection.downgrade()),
            ),
            spawn(
                self.clone()
                    .handle_datagrams(datagrams, connection.downgrade()),
            ),
        ])
        .await;
        authenticator.logout(id);
        Ok(())
    }

    async fn handle_bi_streams(
        self,
        mut bi_streams: IncomingBiStreams,
        connection: WeakConnection,
    ) {
        while let Some(Ok((send, recv))) = bi_streams.next().await {
            if let Some(connection) = connection.upgrade() {
                spawn(self.clone().recv_bi_stream(send, recv, connection));
            }
        }
    }

    async fn handle_uni_streams(
        self,
        mut uni_streams: IncomingUniStreams,
        connection: WeakConnection,
    ) {
        while let Some(Ok(recv)) = uni_streams.next().await {
            if let Some(connection) = connection.upgrade() {
                spawn(self.clone().recv_uni_stream(recv, connection));
            }
        }
    }

    async fn handle_datagrams(self, mut datagrams: Datagrams, connection: WeakConnection) {
        while let Some(Ok(datagram)) = datagrams.next().await {
            if let Some(connection) = connection.upgrade() {
                spawn(self.clone().recv_datagram(datagram, connection));
            }
        }
    }

    async fn recv_bi_stream(self, send: SendStream, recv: RecvStream, connection: Connection) {
        let mut reader = match connection.read(recv).await {
            Ok(reader) => reader,
            Err(error) => {
                warn!("Reading receive stream failed: {}", error);
                return;
            }
        };
        let port = match reader.read() {
            Ok(port) => port,
            Err(error) => {
                warn!("Reading destination port failed: {}", error);
                return;
            }
        };
        let ports = self.0.ports.read().unwrap();
        ports[&port].recv_bi_stream(reader, send, connection)
    }

    async fn recv_uni_stream(self, recv: RecvStream, connection: Connection) {
        let mut reader = match connection.read(recv).await {
            Ok(reader) => reader,
            Err(error) => {
                warn!("Reading receive stream failed: {}", error);
                return;
            }
        };
        let port = match reader.read() {
            Ok(port) => port,
            Err(error) => {
                warn!("Reading destination port failed: {}", error);
                return;
            }
        };
        let ports = self.0.ports.read().unwrap();
        ports[&port].recv_uni_stream(reader, connection)
    }

    async fn recv_datagram(self, datagram: Bytes, connection: Connection) {
        let mut reader = Reader::new(datagram);
        let port = match reader.read() {
            Ok(port) => port,
            Err(error) => {
                warn!("Reading destination port failed: {}", error);
                return;
            }
        };
        let ports = self.0.ports.read().unwrap();
        ports[&port].recv_datagram(reader, connection)
    }
}

#[derive(Default)]
struct HostInner {
    ports: RwLock<HashMap<u16, Box<dyn Receiver>>>,
}

pub(super) struct HostBinding {
    host: Host,
    port: u16,
}

impl HostBinding {
    pub(super) fn port(&self, connection: &Connection) -> u16 {
        assert!(self.host.is_same(connection.host()));
        self.port
    }
}

impl Drop for HostBinding {
    fn drop(&mut self) {
        let mut ports = self.host.0.ports.write().unwrap();
        ports.remove(&self.port);
    }
}
