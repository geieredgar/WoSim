use std::sync::Arc;

use crate::{FilterMapSender, MapSender, Sender};

pub struct Address<M: 'static>(Arc<dyn Sender<M>>);

impl<M: 'static> Address<M> {
    pub fn new(sender: Arc<dyn Sender<M>>) -> Self {
        Self(sender)
    }

    pub fn send(&self, message: M) {
        self.0.send(message);
    }

    pub fn map<N: Send + Sync, F: Send + Sync + 'static + Fn(N) -> M>(
        self,
        transform: F,
    ) -> Address<N> {
        Address::new(Arc::new(MapSender::new(self, transform)))
    }

    pub fn filter_map<N: Send + Sync, F: Send + Sync + 'static + Fn(N) -> Option<M>>(
        self,
        transform: F,
    ) -> Address<N> {
        Address::new(Arc::new(FilterMapSender::new(self, transform)))
    }
}

impl<M: 'static> Clone for Address<M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
