use std::{error::Error, fmt::Debug, sync::Arc};

use log::error;

pub type Dispatcher<M> = dyn Fn(M) -> Result<(), Box<dyn Error>> + Send + Sync;

pub struct Address<M: 'static>(Arc<Dispatcher<M>>);

impl<M: 'static> Address<M> {
    pub fn new(f: impl Fn(M) -> Result<(), Box<dyn Error>> + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    pub fn send(&self, message: M) {
        if let Err(error) = self.try_send(message) {
            error!("Failed sending to address {:?}, caused by: {}", self, error);
        }
    }

    pub fn try_send(&self, message: M) -> Result<(), Box<dyn Error>> {
        (self.0)(message)
    }

    pub fn map<N: Send + Sync + 'static, F: Send + Sync + 'static + Fn(N) -> M>(
        self,
        transform: F,
    ) -> Address<N> {
        Address::new(move |message| self.try_send(transform(message)))
    }

    pub fn filter_map<N: Send + Sync + 'static, F: Send + Sync + 'static + Fn(N) -> Option<M>>(
        self,
        transform: F,
    ) -> Address<N> {
        Address::new(move |message| {
            if let Some(message) = transform(message) {
                self.try_send(message)
            } else {
                Ok(())
            }
        })
    }
}

impl<M: 'static> Debug for Address<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:p}", self.0)
    }
}

impl<M: 'static> Clone for Address<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
