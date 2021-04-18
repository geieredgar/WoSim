mod buffer;
mod command;
mod descriptor;
mod device;
mod error;
mod handle;
mod image;
mod instance;
mod object;
mod physical_device;
mod surface;
mod swapchain;
mod version;

use std::ffi::CStr;

use ash::vk;

pub use buffer::*;
pub use command::*;
pub use descriptor::*;
pub use device::*;
pub use error::*;
pub use handle::*;
pub use image::*;
pub use instance::*;
pub use object::*;
pub use physical_device::*;
pub use surface::*;
pub use swapchain::*;
pub use version::*;

pub use ash::{
    prelude::VkResult,
    vk::{
        AccessFlags, AttachmentDescription, AttachmentLoadOp, AttachmentReference,
        AttachmentStoreOp, BlendFactor, BlendOp, Bool32, BufferCopy, BufferCreateInfo,
        BufferImageCopy, BufferMemoryBarrier, BufferUsageFlags, ClearColorValue,
        ClearDepthStencilValue, ClearValue, ColorComponentFlags, ColorSpaceKHR, CommandBufferLevel,
        CommandBufferUsageFlags, CommandPoolCreateFlags, CommandPoolResetFlags, CompareOp,
        ComponentMapping, ComponentSwizzle, ComputePipelineCreateInfo, CopyDescriptorSet,
        CullModeFlags, DependencyFlags, DescriptorBufferInfo, DescriptorImageInfo,
        DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags, DescriptorType,
        DrawIndexedIndirectCommand, DynamicState, ExtensionProperties, Extent2D, Extent3D,
        FenceCreateFlags, Filter, Format, FormatFeatureFlags, FramebufferCreateFlags, FrontFace,
        GraphicsPipelineCreateInfo, ImageAspectFlags, ImageCreateInfo, ImageLayout,
        ImageMemoryBarrier, ImageSubresourceLayers, ImageSubresourceRange, ImageTiling, ImageType,
        ImageUsageFlags, ImageViewCreateFlags, ImageViewCreateInfo, ImageViewType, IndexType,
        KhrPortabilitySubsetFn, LogicOp, MemoryBarrier, MemoryPropertyFlags, Offset2D,
        PipelineBindPoint, PipelineCacheCreateFlags, PipelineColorBlendAttachmentState,
        PipelineColorBlendStateCreateInfo, PipelineDepthStencilStateCreateInfo,
        PipelineDynamicStateCreateInfo, PipelineInputAssemblyStateCreateInfo,
        PipelineLayoutCreateFlags, PipelineMultisampleStateCreateInfo,
        PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo, PipelineStageFlags,
        PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode,
        PresentModeKHR, PrimitiveTopology, PushConstantRange, QueryPipelineStatisticFlags,
        QueryResultFlags, QueryType, QueueFamilyProperties, QueueFlags, Rect2D,
        RenderPassCreateInfo, SampleCountFlags, SamplerAddressMode, SamplerCreateInfo,
        SamplerMipmapMode, SamplerReductionMode, SamplerReductionModeCreateInfo,
        ShaderModuleCreateFlags, ShaderStageFlags, SharingMode, SpecializationInfo,
        SpecializationMapEntry, SubmitInfo, SubpassContents, SubpassDependency, SubpassDescription,
        SurfaceFormatKHR, VertexInputAttributeDescription, VertexInputBindingDescription,
        VertexInputRate, Viewport, WriteDescriptorSet, FALSE, LOD_CLAMP_NONE, SUBPASS_EXTERNAL,
        TRUE, WHOLE_SIZE,
    },
};

pub use vk_mem::{AllocationCreateFlags, AllocationCreateInfo, AllocationInfo, MemoryUsage};

pub use bytemuck::Pod;

pub type ApiResult = vk::Result;

pub fn contains_extension(extensions: &[ExtensionProperties], extension_name: &CStr) -> bool {
    for extension in extensions {
        if extension_name == unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) } {
            return true;
        }
    }
    false
}
