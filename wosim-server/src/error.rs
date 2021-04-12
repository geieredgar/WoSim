use wosim_common::vulkan::{self, ApiResult};

#[derive(Debug)]
pub enum Error {
    Vulkan(vulkan::Error),
    NoSuitableDeviceFound,
}

impl From<vulkan::Error> for Error {
    fn from(error: vulkan::Error) -> Self {
        Error::Vulkan(error)
    }
}

impl From<ApiResult> for Error {
    fn from(result: ApiResult) -> Self {
        Error::Vulkan(result.into())
    }
}
