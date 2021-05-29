use tokio::sync::mpsc::{self, error::SendError};

use crate::{remote, send, Message};

#[derive(Debug)]
pub struct Connection<M: Message> {
    sender: mpsc::Sender<M>,
    remote: Option<send::Connection>,
}

impl<M: Message> Connection<M> {
    pub fn local(sender: mpsc::Sender<M>) -> Self {
        Self {
            sender,
            remote: None,
        }
    }

    pub fn remote(remote: send::Connection) -> Self {
        Self {
            sender: remote.clone().into_asynchronous(),
            remote: Some(remote),
        }
    }

    pub async fn send(&self, message: M) -> Result<(), SendError<M>> {
        self.sender.send(message).await
    }

    pub fn synchronous(&self) -> mpsc::Sender<M> {
        if let Some(remote) = self.remote.as_ref() {
            remote.clone().into_synchronous()
        } else {
            self.sender.clone()
        }
    }

    pub fn stats(&self) -> Option<remote::ConnectionStats> {
        self.remote.as_ref().map(send::Connection::stats)
    }
}

impl<M: Message + 'static> Clone for Connection<M> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            remote: self.remote.clone(),
        }
    }
}
