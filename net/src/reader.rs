use std::io::Cursor;

use bincode::{deserialize_from, Error};
use bytes::Bytes;
use quinn::{ReadToEndError, RecvStream};
use serde::de::DeserializeOwned;

pub struct Reader(Cursor<Bytes>);

impl From<Bytes> for Reader {
    fn from(bytes: Bytes) -> Self {
        Self(Cursor::new(bytes))
    }
}

pub type ReadError = Error;

impl Reader {
    pub async fn recv(recv: RecvStream, size_limit: usize) -> Result<Self, ReadToEndError> {
        let bytes: Bytes = recv.read_to_end(size_limit).await?.into();
        Ok(bytes.into())
    }

    pub fn read<T: DeserializeOwned>(&mut self) -> Result<T, ReadError> {
        deserialize_from(&mut self.0)
    }
}
