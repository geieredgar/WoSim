use std::{error::Error, fmt::Display};

use bytes::Bytes;
use quinn::{ReadToEndError, RecvStream, SendStream};

use crate::Reader;

pub trait FromDatagram: Sized {
    type Error: Error;

    fn from(reader: Reader) -> Result<Self, Self::Error>;
}

pub trait FromUniStream: Sized {
    type Error: Error;

    fn from(reader: Reader) -> Result<Self, Self::Error>;

    fn size_limit() -> usize;
}

pub trait FromBiStream: Sized {
    type Error: Error;

    fn from(reader: Reader, send: SendStream) -> Result<Self, Self::Error>;

    fn size_limit() -> usize;
}

pub async fn from_uni_stream<T: FromUniStream>(
    recv: RecvStream,
) -> Result<T, ReadOrConvertError<T::Error>> {
    let reader = Reader::recv(recv, T::size_limit())
        .await
        .map_err(ReadOrConvertError::Read)?;
    T::from(reader).map_err(ReadOrConvertError::Convert)
}

pub async fn from_bi_stream<T: FromBiStream>(
    recv: RecvStream,
    send: SendStream,
) -> Result<T, ReadOrConvertError<T::Error>> {
    let reader = Reader::recv(recv, T::size_limit())
        .await
        .map_err(ReadOrConvertError::Read)?;
    T::from(reader, send).map_err(ReadOrConvertError::Convert)
}

pub fn from_datagram<T: FromDatagram>(datagram: Bytes) -> Result<T, T::Error> {
    T::from(datagram.into())
}

#[derive(Debug)]
pub enum ReadOrConvertError<E: Error> {
    Read(ReadToEndError),
    Convert(E),
}

impl<E: Error> Display for ReadOrConvertError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(error) => write!(f, "Read error: {}", error),
            Self::Convert(error) => write!(f, "Convert error: {}", error),
        }
    }
}

impl<E: Error> Error for ReadOrConvertError<E> {}
