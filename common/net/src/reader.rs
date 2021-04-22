use std::io::Cursor;

use bincode::{deserialize_from, Error};
use bytes::Bytes;
use serde::de::DeserializeOwned;

pub struct Reader(Cursor<Bytes>);

impl Reader {
    pub fn new(inner: Bytes) -> Self {
        Self(Cursor::new(inner))
    }

    pub fn read<T: DeserializeOwned>(&mut self) -> Result<T, Error> {
        deserialize_from(&mut self.0)
    }
}
