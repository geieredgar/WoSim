use std::{fmt::Display, io};

use actor::SendError;
use semver::{ReqParseError, SemVerError};
use vulkan::ApiResult;
use winit::error::{ExternalError, OsError};

use crate::resolver::ResolveError;

#[derive(Debug)]
pub enum Error {
    Vulkan(vulkan::Error),
    Os(OsError),
    Io(io::Error),
    Egui(super::egui::Error),
    External(ExternalError),
    NoSuitableDeviceFound,
    NoSuitableSurfaceFormat,
    NoSuitablePresentMode,
    Json(serde_json::Error),
    SemVer(SemVerError),
    ReqParse(ReqParseError),
    Resolve(ResolveError),
    Send(SendError),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<vulkan::Error> for Error {
    fn from(error: vulkan::Error) -> Self {
        Self::Vulkan(error)
    }
}

impl From<OsError> for Error {
    fn from(error: OsError) -> Self {
        Self::Os(error)
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

impl From<super::egui::Error> for Error {
    fn from(error: super::egui::Error) -> Self {
        Self::Egui(error)
    }
}

impl From<ExternalError> for Error {
    fn from(error: ExternalError) -> Self {
        Self::External(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<ReqParseError> for Error {
    fn from(error: ReqParseError) -> Self {
        Self::ReqParse(error)
    }
}

impl From<SemVerError> for Error {
    fn from(error: SemVerError) -> Self {
        Self::SemVer(error)
    }
}

impl From<ResolveError> for Error {
    fn from(error: ResolveError) -> Self {
        Self::Resolve(error)
    }
}
