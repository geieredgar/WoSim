use std::{error::Error, fmt::Display, io};

use bytes::Bytes;
use quinn::ReadExactError;

#[derive(Debug)]
pub enum RecvError {
    FromIncoming(Box<dyn Error>),
    ReadRequestId(io::Error),
    ReadRequestSize(io::Error),
    ReadRequestData(ReadExactError),
    ReadResponseKey(io::Error),
    RequestTooLarge { size: usize, size_limit: usize },
    InvalidResponseKey(u32),
    ReadResponseSize(io::Error),
    ResponseTooLarge { size: usize, size_limit: usize },
    ReadResponseData(ReadExactError),
    SendResponseBytes(Bytes),
}

impl Display for RecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
