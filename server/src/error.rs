use wosim_common_vulkan::ApiResult;

#[derive(Debug)]
pub enum Error {
    Vulkan(wosim_common_vulkan::Error),
    NoSuitableDeviceFound,
}

impl From<wosim_common_vulkan::Error> for Error {
    fn from(error: wosim_common_vulkan::Error) -> Self {
        Self::Vulkan(error)
    }
}

impl From<ApiResult> for Error {
    fn from(result: ApiResult) -> Self {
        Self::Vulkan(result.into())
    }
}
