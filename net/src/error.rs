use quinn::{ConnectError, ConnectionError, ReadToEndError, SendDatagramError, WriteError};

#[derive(Debug)]
pub enum EstablishConnectionError {
    Connect(ConnectError),
    Connection(ConnectionError),
    Deserialize(bincode::Error),
    ReadToEnd(ReadToEndError),
    SendDatagram(SendDatagramError),
    Serialize(bincode::Error),
    TokenMissing,
    InvalidToken,
    TokenRejected(String),
    Write(WriteError),
}

impl From<ConnectError> for EstablishConnectionError {
    fn from(error: ConnectError) -> Self {
        Self::Connect(error)
    }
}

impl From<ConnectionError> for EstablishConnectionError {
    fn from(error: ConnectionError) -> Self {
        Self::Connection(error)
    }
}

impl From<ReadToEndError> for EstablishConnectionError {
    fn from(error: ReadToEndError) -> Self {
        Self::ReadToEnd(error)
    }
}

impl From<SendDatagramError> for EstablishConnectionError {
    fn from(error: SendDatagramError) -> Self {
        Self::SendDatagram(error)
    }
}

impl From<WriteError> for EstablishConnectionError {
    fn from(error: WriteError) -> Self {
        Self::Write(error)
    }
}
