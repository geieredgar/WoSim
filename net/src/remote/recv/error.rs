use std::{error::Error as StdError, fmt::Debug};

use eyre::eyre;
use quinn::{ReadError, ReadExactError, WriteError};
use tokio::sync::mpsc::error::SendError;

pub enum Error {
    Error(eyre::Report),
    Closed,
}

pub trait ConvertErr<T, E> {
    fn convert_err(self, message: &'static str) -> Result<T, Error>;
}

impl<T, E: Into<Error>> ConvertErr<T, E> for Result<T, E> {
    fn convert_err(self, message: &'static str) -> Result<T, Error> {
        match self.map_err(Into::into) {
            Ok(value) => Ok(value),
            Err(Error::Closed) => Err(Error::Closed),
            Err(Error::Error(error)) => Err(Error::Error(error.wrap_err(message))),
        }
    }
}

impl From<quinn::ConnectionError> for Error {
    fn from(error: quinn::ConnectionError) -> Self {
        if let quinn::ConnectionError::LocallyClosed = error {
            Self::Closed
        } else {
            Self::Error(error.into())
        }
    }
}

impl From<ReadError> for Error {
    fn from(error: ReadError) -> Self {
        if let ReadError::ConnectionClosed(quinn::ConnectionError::LocallyClosed) = error {
            Self::Closed
        } else {
            Self::Error(error.into())
        }
    }
}

impl From<WriteError> for Error {
    fn from(error: WriteError) -> Self {
        if let WriteError::ConnectionClosed(quinn::ConnectionError::LocallyClosed) = error {
            Self::Closed
        } else {
            Self::Error(error.into())
        }
    }
}

impl From<ReadExactError> for Error {
    fn from(error: ReadExactError) -> Self {
        if let ReadExactError::ReadError(ReadError::ConnectionClosed(
            quinn::ConnectionError::LocallyClosed,
        )) = error
        {
            Self::Closed
        } else {
            Self::Error(error.into())
        }
    }
}

impl From<Box<dyn StdError + Send + Sync + 'static>> for Error {
    fn from(error: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Self::Error(eyre!(error))
    }
}

impl From<bincode::Error> for Error {
    fn from(error: bincode::Error) -> Self {
        Self::Error(eyre!(error))
    }
}

impl<M: Debug + Send + Sync + 'static> From<SendError<M>> for Error {
    fn from(error: SendError<M>) -> Self {
        Self::Error(error.into())
    }
}
