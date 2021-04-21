use actor::Sender;
use log::warn;
use tokio::spawn;

use crate::{Connection, Protocol, Writer};

pub(super) struct RemoteSender {
    connection: Connection,
    port: u16,
}

impl RemoteSender {
    pub(super) fn new(connection: Connection, port: u16) -> Self {
        Self { connection, port }
    }
}

impl<T: Protocol> Sender<T> for RemoteSender {
    fn send(&self, message: T) {
        let connection = self.connection.clone();
        let port = self.port;
        spawn(async move {
            let mut writer = Writer::default();
            if let Err(error) = writer.write(&port) {
                warn!("Writing port number failed: {}", error);
            }
            if let Err(error) = message.send(writer, connection.clone()) {
                warn!("Sending message failed: {}", error)
            };
        });
    }
}
