use std::ffi::CStr;

use ash::{
    extensions::khr,
    vk::{PresentModeKHR, SurfaceFormatKHR},
};

pub struct SwapchainConfiguration {
    pub present_mode: PresentModeKHR,
    pub surface_format: SurfaceFormatKHR,
}

pub struct Swapchain {}

impl Swapchain {
    pub fn extension_name() -> &'static CStr {
        khr::Swapchain::name()
    }
}
