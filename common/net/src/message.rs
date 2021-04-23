use std::error::Error;

use quinn::Connection;

use crate::{FromBiStream, FromDatagram, FromUniStream};

pub trait Message:
    FromDatagram + FromUniStream + FromBiStream + Sized + Send + Sync + 'static
{
    type Error: Error;

    fn send(self, connection: Connection) -> Result<(), <Self as Message>::Error>;
}

pub enum SessionMessage<I, M> {
    Connect(I),
    Message(I, M),
    Disconnect(I),
}
