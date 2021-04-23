use actor::{Address, Sender};
use log::warn;
use quinn::Connection;
use tokio::spawn;

use crate::{Message, SessionMessage};

pub(super) struct RemoteSender(pub(super) Connection);

impl<T: Message> Sender<T> for RemoteSender {
    fn send(&self, message: T) {
        let connection = self.0.clone();
        spawn(async move {
            if let Err(error) = message.send(connection) {
                warn!("Sending message failed: {}", error)
            };
        });
    }
}

pub(super) struct LocalSender<I: Clone + 'static, M: 'static>(Address<SessionMessage<I, M>>, I);

impl<I: Clone + 'static, M: 'static> LocalSender<I, M> {
    pub(super) fn new(address: Address<SessionMessage<I, M>>, identity: I) -> Self {
        address.send(SessionMessage::Connect(identity.clone()));
        Self(address, identity)
    }
}

impl<I: Clone + Send + Sync + 'static, M: 'static> Sender<M> for LocalSender<I, M> {
    fn send(&self, message: M) {
        self.0
            .send(SessionMessage::Message(self.1.clone(), message))
    }
}

impl<I: Clone + 'static, M: 'static> Drop for LocalSender<I, M> {
    fn drop(&mut self) {
        self.0.send(SessionMessage::Disconnect(self.1.clone()))
    }
}
