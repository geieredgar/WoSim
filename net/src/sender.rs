use log::warn;
use quinn::{Connection, VarInt};
use tokio::spawn;

use crate::Message;

pub(super) struct RemoteSender(pub(super) Connection);

impl RemoteSender {
    pub(super) fn send(&self, message: impl Message) {
        let connection = self.0.clone();
        spawn(async move {
            if let Err(error) = message.send(connection) {
                warn!("Sending message failed: {}", error)
            };
        });
    }
}

impl Drop for RemoteSender {
    fn drop(&mut self) {
        self.0.close(VarInt::from_u32(0), &[]);
    }
}
