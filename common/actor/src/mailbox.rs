use std::sync::Arc;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use crate::Address;

pub type Mailbox<T> = UnboundedReceiver<T>;

pub fn mailbox<T: Send + 'static>() -> (Mailbox<T>, Address<T>) {
    let (send, recv) = unbounded_channel();
    (recv, Address::new(Arc::new(send)))
}
