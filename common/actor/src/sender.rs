use log::warn;
use tokio::sync::mpsc::UnboundedSender;

pub trait Sender<T>: Send + Sync + 'static {
    fn send(&self, message: T);
}

impl<T: Send + 'static> Sender<T> for UnboundedSender<T> {
    fn send(&self, message: T) {
        match self.send(message) {
            Ok(()) => {}
            Err(error) => {
                warn!("Sending failed. Receiver already closed. Error: {}", error)
            }
        }
    }
}
