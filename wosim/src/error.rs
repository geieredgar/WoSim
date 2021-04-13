use winit::error::OsError;
use wosim_common::vulkan::{self, ApiResult};

#[derive(Debug)]
pub enum Error {
    Vulkan(vulkan::Error),
    Os(OsError),
    NoSuitableDeviceFound,
    NoSuitableSurfaceFormat,
    NoSuitablePresentMode,
}

impl From<vulkan::Error> for Error {
    fn from(error: vulkan::Error) -> Self {
        Error::Vulkan(error)
    }
}

impl From<OsError> for Error {
    fn from(error: OsError) -> Self {
        Error::Os(error)
    }
}

impl From<ApiResult> for Error {
    fn from(result: ApiResult) -> Self {
        Error::Vulkan(result.into())
    }
}