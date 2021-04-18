use std::io;

use winit::error::OsError;
use wosim_common::vulkan::{self, ApiResult};

#[derive(Debug)]
pub enum Error {
    Vulkan(vulkan::Error),
    Os(OsError),
    Io(io::Error),
    Egui(super::egui::Error),
    NoSuitableDeviceFound,
    NoSuitableSurfaceFormat,
    NoSuitablePresentMode,
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
