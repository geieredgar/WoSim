use serde::Serialize;
use tokio::sync::oneshot::Sender;

pub enum Return<T> {
    Local(Sender<T>),
    Remote(RemoteReturn),
}

impl<T: Serialize + Send + 'static> Return<T> {
    pub fn send(self, value: T) -> Result<(), T> {
        match self {
            Return::Local(send) => send.send(value),
            Return::Remote(ret) => {
                ret.0.send(value);
                Ok(())
            }
        }
    }
}

impl<T: Serialize + Send + 'static> From<Sender<T>> for Return<T> {
    fn from(sender: Sender<T>) -> Self {
        Return::Local(sender)
    }
}

pub struct RemoteReturn(pub(crate) crate::Sender);
