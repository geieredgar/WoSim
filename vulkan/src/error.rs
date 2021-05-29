use ash::{InstanceError, LoadingError};
use thiserror::Error;

use super::ApiResult;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not load vulkan library")]
    Loading(#[from] LoadingError),
    #[error(transparent)]
    Instance(#[from] InstanceError),
    #[error("api call failed")]
    ApiResult(#[from] ApiResult),
    #[error(transparent)]
    Memory(#[from] vk_mem::Error),
}
