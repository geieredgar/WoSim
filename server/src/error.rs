use vulkan::ApiResult;

#[derive(Debug)]
pub enum Error {
    Vulkan(vulkan::Error),
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
