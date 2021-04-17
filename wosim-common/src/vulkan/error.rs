use ash::{InstanceError, LoadingError};

use super::ApiResult;

#[derive(Debug)]
pub enum Error {
    Loading(LoadingError),
    Instance(InstanceError),
    ApiResult(ApiResult),
    Memory(vk_mem::Error),
}

impl From<LoadingError> for Error {
    fn from(error: LoadingError) -> Self {
        Self::Loading(error)
    }
}

impl From<InstanceError> for Error {
    fn from(error: InstanceError) -> Self {
        Self::Instance(error)
    }
}

impl From<ApiResult> for Error {
    fn from(result: ApiResult) -> Self {
        Self::ApiResult(result)
    }
}

impl From<vk_mem::Error> for Error {
    fn from(error: vk_mem::Error) -> Self {
        Self::Memory(error)
    }
}
