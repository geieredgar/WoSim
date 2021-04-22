use std::error::Error;

use quinn::SendStream;

use super::{Connection, Reader, Writer};

pub trait Protocol: Sized + Send + Sync + 'static {
    type Error: Error;

    fn send(self, writer: Writer, connection: Connection) -> Result<(), Self::Error>;
    fn recv_uni_stream(reader: Reader, connection: Connection) -> Result<Self, Self::Error>;
    fn recv_bi_stream(
        reader: Reader,
        send: SendStream,
        connection: Connection,
    ) -> Result<Self, Self::Error>;
    fn recv_datagram(reader: Reader, connection: Connection) -> Result<Self, Self::Error>;
}
