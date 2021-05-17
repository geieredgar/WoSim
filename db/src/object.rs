use std::io::{self, Read, Write};

use crate::{header::Format, reference::DatabaseRef};

pub trait Object: Sized {
    fn format() -> Format;

    fn serialize(&mut self, writer: &mut impl Write) -> io::Result<()>;

    fn deserialize(reader: &mut impl Read, database: DatabaseRef) -> io::Result<Self>;
}
