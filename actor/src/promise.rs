use tokio::sync::oneshot::{channel, Receiver, Sender};

pub type Promise<T> = Receiver<T>;
pub type Return<T> = Sender<T>;

pub fn promise<T>() -> (Promise<T>, Return<T>) {
    let (send, recv) = channel();
    (recv, send)
}
