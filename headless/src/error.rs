use net::{recv::OpenError, FromPemError, SelfSignError};
use vulkan::ApiResult;

use std::io;

use server::CreateServiceError;

#[derive(Debug)]
pub enum Error {
    Vulkan(vulkan::Error),
    Io(io::Error),
    OpenServer(OpenError),
    NoSuitableDeviceFound,
    CreateService(CreateServiceError),
    SelfSign(SelfSignError),
    FromPem(FromPemError),
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

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
