use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use tokio::sync::oneshot::Sender;

#[derive(Clone)]
pub struct Return(Arc<Mutex<ReturnInner>>);

struct ReturnInner {
    waiters: HashMap<u32, (Sender<Bytes>, usize)>,
}

impl Return {
    pub fn wait(&self, key: u32, tx: Sender<Bytes>, limit: usize) {
        let mut inner = self.0.lock().unwrap();
        inner.waiters.insert(key, (tx, limit));
    }

    pub fn wake(&self, key: u32) -> Option<(Sender<Bytes>, usize)> {
        let mut inner = self.0.lock().unwrap();
        inner.waiters.remove(&key)
    }
}

impl Default for Return {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(ReturnInner {
            waiters: HashMap::new(),
        })))
    }
}
