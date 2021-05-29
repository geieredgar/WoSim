use serde::Serialize;
use tokio::sync::oneshot;

use crate::recv;

pub enum Return<T> {
    Local(oneshot::Sender<T>),
    Remote(recv::Return),
}

impl<T: Serialize + Send + 'static> Return<T> {
    pub fn send(self, value: T) -> Result<(), T> {
        match self {
            Return::Local(tx) => tx.send(value),
            Return::Remote(r#return) => {
                r#return.r#return(value);
                Ok(())
            }
        }
    }
}

impl<T: Serialize + Send + 'static> From<oneshot::Sender<T>> for Return<T> {
    fn from(sender: oneshot::Sender<T>) -> Self {
        Return::Local(sender)
    }
}
