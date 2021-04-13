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
        AccessFlags, AttachmentDescription, AttachmentLoadOp, AttachmentReference,
        AttachmentStoreOp, BlendFactor, BlendOp, BufferMemoryBarrier, ClearColorValue,
        ClearDepthStencilValue, ClearValue, ColorComponentFlags, ColorSpaceKHR, CommandBufferLevel,
        CommandBufferUsageFlags, CommandPoolCreateFlags, CommandPoolResetFlags, CompareOp,
        CullModeFlags, DependencyFlags, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags,
        DescriptorType, ExtensionProperties, Extent2D, FenceCreateFlags, Format,
        FormatFeatureFlags, FramebufferCreateFlags, FrontFace, GraphicsPipelineCreateInfo,
        ImageAspectFlags, ImageLayout, ImageMemoryBarrier, ImageSubresourceRange, ImageTiling,
        KhrPortabilitySubsetFn, LogicOp, MemoryBarrier, Offset2D, PhysicalDeviceProperties,
        PipelineBindPoint, PipelineCacheCreateFlags, PipelineColorBlendAttachmentState,
        PipelineColorBlendStateCreateInfo, PipelineDepthStencilStateCreateInfo,
        PipelineInputAssemblyStateCreateInfo, PipelineLayoutCreateFlags,
        PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
        PipelineShaderStageCreateInfo, PipelineStageFlags, PipelineVertexInputStateCreateInfo,
        PipelineViewportStateCreateInfo, PolygonMode, PresentModeKHR, PrimitiveTopology,
        QueueFamilyProperties, QueueFlags, Rect2D, RenderPassCreateInfo, SampleCountFlags,
        ShaderModuleCreateFlags, ShaderStageFlags, SubmitInfo, SubpassContents, SubpassDependency,
        SubpassDescription, SurfaceFormatKHR, VertexInputAttributeDescription,
        VertexInputBindingDescription, VertexInputRate, Viewport, SUBPASS_EXTERNAL,
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
