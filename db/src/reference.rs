use std::{
    io,
    sync::{Arc, RwLock},
};

use crate::{file::FileHeader, lock::Lock, raw::RawDatabase};

#[derive(Clone)]
pub struct DatabaseRef(Arc<RwLock<RawDatabase>>);

impl DatabaseRef {
    pub(crate) fn new(raw: RawDatabase) -> Self {
        Self(Arc::new(RwLock::new(raw)))
    }

    pub(crate) fn lock(&self) -> Lock<'_> {
        Lock::new(self.0.read().unwrap())
    }

    pub(crate) fn snapshot(&self, root: FileHeader) -> io::Result<()> {
        self.0.write().unwrap().snapshot(root)
    }
}
