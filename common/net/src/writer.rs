use std::io::Cursor;

use bincode::{serialize_into, Error};
use serde::Serialize;

#[derive(Default)]
pub struct Writer(Cursor<Vec<u8>>);

impl Writer {
    pub fn write<T: Serialize>(&mut self, value: &T) -> Result<(), Error> {
        serialize_into(&mut self.0, value)
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.0.into_inner()
    }
}
