use actor::Address;
use log::error;
use quinn::SendStream;

use crate::{Connection, Protocol, Reader};

pub trait Receiver: Send + Sync + 'static {
    fn recv_bi_stream(&self, reader: Reader, send: SendStream, connection: Connection);
    fn recv_uni_stream(&self, reader: Reader, connection: Connection);
    fn recv_datagram(&self, reader: Reader, connection: Connection);
}

impl<M: Protocol + 'static> Receiver for Address<M> {
    fn recv_bi_stream(&self, reader: Reader, send: SendStream, connection: Connection) {
        let message = match M::recv_bi_stream(reader, send, connection) {
            Ok(message) => message,
            Err(error) => {
                error!(
                    "Receiving message from bidirectional stream failed: {}",
                    error
                );
                return;
            }
        };
        self.send(message)
    }

    fn recv_uni_stream(&self, reader: Reader, connection: Connection) {
        let message = match M::recv_uni_stream(reader, connection) {
            Ok(message) => message,
            Err(error) => {
                error!(
                    "Receiving message from unidirectional stream failed: {}",
                    error
                );
                return;
            }
        };
        self.send(message)
    }

    fn recv_datagram(&self, reader: Reader, connection: Connection) {
        let message = match M::recv_uni_stream(reader, connection) {
            Ok(message) => message,
            Err(error) => {
                error!("Receiving message from datagram failed: {}", error);
                return;
            }
        };
        self.send(message)
    }
}
