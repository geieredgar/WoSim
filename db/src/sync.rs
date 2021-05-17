use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Barrier,
    },
    thread::{spawn, Builder, JoinHandle},
};

use crate::mmap::MappedFile;

pub struct Synchronizer {
    state: Arc<State>,
    handle: Option<JoinHandle<()>>,
}

impl Synchronizer {
    pub fn new(data: MappedFile) -> Self {
        let state = Arc::new(State {
            barrier: Barrier::new(2),
            cancelled: AtomicBool::new(false),
        });
        let handle = Some(Self::spawn(state.clone(), data));
        Self { state, handle }
    }

    pub fn sync(&self) {
        self.state.barrier.wait();
    }

    fn spawn(state: Arc<State>, data: MappedFile) -> JoinHandle<()> {
        Builder::new().name("database synchronization thread".into());
        spawn(move || loop {
            state.barrier.wait();
            if state.cancelled.load(Ordering::SeqCst) {
                return;
            }
            data.sync().unwrap()
        })
    }
}

impl Drop for Synchronizer {
    fn drop(&mut self) {
        self.state.cancelled.store(true, Ordering::SeqCst);
        self.state.barrier.wait();
        self.handle.take().unwrap().join().unwrap();
    }
}

struct State {
    barrier: Barrier,
    cancelled: AtomicBool,
}
