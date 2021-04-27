use std::io::Cursor;

use bincode::{serialize_into, Error};
use quinn::{SendStream, WriteError};
use serde::Serialize;

#[derive(Default)]
pub struct Writer(Cursor<Vec<u8>>);

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write<T: Serialize>(&mut self, value: &T) -> Result<(), Error> {
        serialize_into(&mut self.0, value)
    }

    pub async fn send(self, mut send: SendStream) -> Result<(), WriteError> {
        send.write_all(&self.0.into_inner()).await?;
        send.finish().await?;
        Ok(())
    }
}
