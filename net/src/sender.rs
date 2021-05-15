use std::error::Error;

use actor::Address;
use log::warn;
use quinn::{Connection, VarInt};
use tokio::spawn;

use crate::{Message, SessionMessage};

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

pub(super) struct LocalSender<I: Clone + 'static, M: 'static>(Address<SessionMessage<I, M>>, I);

impl<I: Clone + 'static, M: 'static> LocalSender<I, M> {
    pub(super) fn new(address: Address<SessionMessage<I, M>>, identity: I) -> Self {
        address.send(SessionMessage::Connect(identity.clone()));
        Self(address, identity)
    }

    pub(super) fn try_send(&self, message: M) -> Result<(), Box<dyn Error>> {
        self.0
            .try_send(SessionMessage::Message(self.1.clone(), message))
    }
}

impl<I: Clone + 'static, M: 'static> Drop for LocalSender<I, M> {
    fn drop(&mut self) {
        self.0.send(SessionMessage::Disconnect(self.1.clone()))
    }
}
