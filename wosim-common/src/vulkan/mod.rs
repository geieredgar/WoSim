mod device;
mod error;
mod handle;
mod instance;
mod object;
mod surface;
mod swapchain;
mod version;

use std::ffi::CStr;

use ash::vk;

pub use device::*;
pub use error::*;
pub use handle::*;
pub use instance::*;
pub use object::*;
pub use surface::*;
pub use swapchain::*;
pub use version::*;

pub use ash::{
    prelude::VkResult,
    vk::{
        AccessFlags, BufferMemoryBarrier, ColorSpaceKHR, CommandBufferLevel,
        CommandBufferUsageFlags, CommandPoolCreateFlags, CommandPoolResetFlags, DependencyFlags,
        ExtensionProperties, Extent2D, FenceCreateFlags, Format, ImageAspectFlags, ImageLayout,
        ImageMemoryBarrier, ImageSubresourceRange, KhrPortabilitySubsetFn, MemoryBarrier,
        PhysicalDeviceProperties, PipelineStageFlags, PresentModeKHR, QueueFamilyProperties,
        QueueFlags, SubmitInfo, SurfaceFormatKHR,
    },
};

pub type ApiResult = vk::Result;

pub fn contains_extension(extensions: &[ExtensionProperties], extension_name: &CStr) -> bool {
    for extension in extensions {
        if extension_name == unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) } {
            return true;
        }
    }
    false
}
