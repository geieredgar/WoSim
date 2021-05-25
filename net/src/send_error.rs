use std::{error::Error, fmt::Debug, fmt::Display, io};

use bytes::Bytes;
use quinn::{ConnectionError, SendDatagramError, WriteError};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum SendError {
    IntoOutgoing(Box<dyn Error>),
    SerializeResponse(bincode::Error),
    WriteRequestId(io::Error),
    WriteRequestSize(io::Error),
    WriteRequestData(WriteError),
    OpenStream(ConnectionError),
    FinishRequest(WriteError),
    SendDatagram(SendDatagramError),
    SendResponsePair(mpsc::error::SendError<(u32, Bytes)>),
    WriteResponseKey(io::Error),
    WriteResponseData(WriteError),
    FinishResponse(WriteError),
    NoResponseSender,
}

impl Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
