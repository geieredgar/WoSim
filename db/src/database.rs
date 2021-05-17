use std::{
    fs::OpenOptions,
    io::{self, Seek, SeekFrom},
    ops::{Deref, DerefMut},
    path::Path,
};

use crate::{file::File, object::Object, raw::RawDatabase, reference::DatabaseRef};

pub struct Database<T: Object> {
    file: File,
    content: T,
    database: DatabaseRef,
}

impl<T: Object> Database<T> {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let (raw, header) = RawDatabase::open(file, &T::format())?;
        let database = DatabaseRef::new(raw);
        let file = File::from_header(header, database.clone());
        let content = T::deserialize(&mut file.read(), database.clone())?;
        Ok(Self {
            file,
            content,
            database,
        })
    }

    pub fn create(
        path: impl AsRef<Path>,
        constructor: impl FnOnce(DatabaseRef) -> T,
    ) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?;
        let raw = RawDatabase::create(file, &T::format())?;
        let database = DatabaseRef::new(raw);
        let file = File::new(database.clone());
        let content = constructor(database.clone());
        Ok(Self {
            file,
            content,
            database,
        })
    }

    pub fn snapshot(&mut self) -> io::Result<()> {
        let mut writer = self.file.write();
        self.content.serialize(&mut writer)?;
        let size = writer.seek(SeekFrom::Current(0))?;
        writer.set_len(size);
        drop(writer);
        self.database.snapshot(self.file.header())
    }
}

impl<T: Object> Deref for Database<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<T: Object> DerefMut for Database<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content
    }
}

impl<T: Object> Drop for Database<T> {
    fn drop(&mut self) {
        self.database.lock().close()
    }
}
