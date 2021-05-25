use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use tokio::sync::oneshot::Sender;

#[derive(Clone)]
pub struct RecvQueue(Arc<Mutex<RecvQueueInner>>);

struct RecvQueueInner {
    senders: HashMap<u32, (Sender<Bytes>, usize)>,
}

impl RecvQueue {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(RecvQueueInner {
            senders: HashMap::new(),
        })))
    }

    pub fn enqueue(&self, key: u32, sender: Sender<Bytes>, limit: usize) {
        let mut inner = self.0.lock().unwrap();
        inner.senders.insert(key, (sender, limit));
    }

    pub fn dequeue(&self, key: u32) -> Option<(Sender<Bytes>, usize)> {
        let mut inner = self.0.lock().unwrap();
        inner.senders.remove(&key)
    }
}
