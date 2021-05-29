use std::error::Error as StdError;

use eyre::eyre;
use quinn::{ApplicationClose, ReadError, ReadExactError, SendDatagramError, WriteError};

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
        if let quinn::ConnectionError::ApplicationClosed(ApplicationClose { error_code, .. }) =
            &error
        {
            if error_code.into_inner() == 0 {
                return Self::Closed;
            }
        }
        Self::Error(error.into())
    }
}

impl From<ReadError> for Error {
    fn from(error: ReadError) -> Self {
        if let ReadError::ConnectionClosed(quinn::ConnectionError::ApplicationClosed(
            ApplicationClose { error_code, .. },
        )) = &error
        {
            if error_code.into_inner() == 0 {
                return Self::Closed;
            }
        }
        Self::Error(error.into())
    }
}

impl From<WriteError> for Error {
    fn from(error: WriteError) -> Self {
        if let WriteError::ConnectionClosed(quinn::ConnectionError::ApplicationClosed(
            ApplicationClose { error_code, .. },
        )) = &error
        {
            if error_code.into_inner() == 0 {
                return Self::Closed;
            }
        }
        Self::Error(error.into())
    }
}

impl From<SendDatagramError> for Error {
    fn from(error: SendDatagramError) -> Self {
        if let SendDatagramError::ConnectionClosed(quinn::ConnectionError::ApplicationClosed(
            ApplicationClose { error_code, .. },
        )) = &error
        {
            if error_code.into_inner() == 0 {
                return Self::Closed;
            }
        }
        Self::Error(error.into())
    }
}

impl From<ReadExactError> for Error {
    fn from(error: ReadExactError) -> Self {
        if let ReadExactError::ReadError(ReadError::ConnectionClosed(
            quinn::ConnectionError::ApplicationClosed(ApplicationClose { error_code, .. }),
        )) = &error
        {
            if error_code.into_inner() == 0 {
                return Self::Closed;
            }
        }
        Self::Error(error.into())
    }
}

impl From<Box<dyn StdError + Send + Sync + 'static>> for Error {
    fn from(error: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Self::Error(eyre!(error))
    }
}
