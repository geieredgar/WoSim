use ash::{InstanceError, LoadingError};

use super::ApiResult;

#[derive(Debug)]
pub enum Error {
    Loading(LoadingError),
    Instance(InstanceError),
    ApiResult(ApiResult),
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
