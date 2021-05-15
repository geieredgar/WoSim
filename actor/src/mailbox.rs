use std::fmt::Debug;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use crate::Address;

pub type Mailbox<T> = UnboundedReceiver<T>;

pub fn mailbox<T: Send + Debug + 'static>() -> (Mailbox<T>, Address<T>) {
    let (send, recv) = unbounded_channel();
    (
        recv,
        Address::new(move |message| send.send(message).map_err(|e| e.into())),
    )
}

pub async fn forward<T: 'static>(mut mailbox: Mailbox<T>, address: Address<T>) {
    while let Some(message) = mailbox.recv().await {
        address.send(message);
    }
}
