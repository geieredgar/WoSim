use vulkan::ApiResult;

use std::{io, net::AddrParseError};

use quinn::{crypto::rustls::TLSError, EndpointError, ParseError};

#[derive(Debug)]
pub enum Error {
    Vulkan(vulkan::Error),
    Io(io::Error),
    Parse(ParseError),
    AddrParse(AddrParseError),
    Tls(TLSError),
    WebPki(webpki::Error),
    Endpoint(EndpointError),
    NoSuitableDeviceFound,
}

impl From<vulkan::Error> for Error {
    fn from(error: vulkan::Error) -> Self {
        Self::Vulkan(error)
    }
}

impl From<ApiResult> for Error {
    fn from(result: ApiResult) -> Self {
        Self::Vulkan(result.into())
    }
}

impl From<webpki::Error> for Error {
    fn from(error: webpki::Error) -> Self {
        Self::WebPki(error)
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ParseError> for Error {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

impl From<TLSError> for Error {
    fn from(error: TLSError) -> Self {
        Self::Tls(error)
    }
}

impl From<AddrParseError> for Error {
    fn from(error: AddrParseError) -> Self {
        Self::AddrParse(error)
    }
}

impl From<EndpointError> for Error {
    fn from(error: EndpointError) -> Self {
        Self::Endpoint(error)
    }
}
