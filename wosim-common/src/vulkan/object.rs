use std::{ops::Deref, sync::Arc};

use ash::{
    prelude::VkResult,
    version::DeviceV1_0,
    vk::{
        self, BufferMemoryBarrier, ClearValue, CommandBufferAllocateInfo, CommandBufferBeginInfo,
        CommandBufferInheritanceInfo, CommandBufferLevel, CommandBufferUsageFlags,
        CommandPoolResetFlags, DependencyFlags, FramebufferCreateFlags, FramebufferCreateInfo,
        GraphicsPipelineCreateInfo, ImageMemoryBarrier, MemoryBarrier, PipelineBindPoint,
        PipelineStageFlags, Rect2D, RenderPassBeginInfo, SubpassContents,
    },
};

use super::{DerefHandle, Device, Handle, HandleWrapper};

pub struct Object<T: Handle> {
    pub(super) device: Arc<Device>,
    pub(super) handle: T,
}

impl<T: Handle> HandleWrapper for Object<T> {
    type Handle = T;
}

impl<T: Handle> Drop for Object<T> {
    fn drop(&mut self) {
        self.device.destroy_handle(self.handle)
    }
}

impl<T: DerefHandle> Deref for Object<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

pub type Fence = Object<vk::Fence>;
pub type CommandPool = Object<vk::CommandPool>;
pub type Semaphore = Object<vk::Semaphore>;
pub type CommandBuffer = Object<vk::CommandBuffer>;
pub type ImageView = Object<vk::ImageView>;
pub type RenderPass = Object<vk::RenderPass>;
pub type ShaderModule = Object<vk::ShaderModule>;
pub type PipelineCache = Object<vk::PipelineCache>;
pub type Pipeline = Object<vk::Pipeline>;
pub type PipelineLayout = Object<vk::PipelineLayout>;
pub type DescriptorSetLayout = Object<vk::DescriptorSetLayout>;
pub type Framebuffer = Object<vk::Framebuffer>;

impl CommandPool {
    pub fn allocate(&self, level: CommandBufferLevel, count: u32) -> VkResult<Vec<CommandBuffer>> {
        let create_info = CommandBufferAllocateInfo::builder()
            .command_pool(self.handle)
            .level(level)
            .command_buffer_count(count);
        Ok(
            unsafe { self.device.inner.allocate_command_buffers(&create_info) }?
                .into_iter()
                .map(|handle| CommandBuffer {
                    handle,
                    device: self.device.clone(),
                })
                .collect(),
        )
    }

    pub fn reset(&self, flags: CommandPoolResetFlags) -> VkResult<()> {
        unsafe { self.device.inner.reset_command_pool(self.handle, flags) }
    }
}

impl Fence {
    pub fn wait(&self) -> VkResult<()> {
        unsafe {
            self.device
                .inner
                .wait_for_fences(&[self.handle], false, u64::MAX)
        }
    }

    pub fn reset(&self) -> VkResult<()> {
        unsafe { self.device.inner.reset_fences(&[self.handle]) }
    }
}

impl CommandBuffer {
    pub fn begin(
        &self,
        flags: CommandBufferUsageFlags,
        inheritance: Option<&CommandBufferInheritanceInfo>,
    ) -> VkResult<()> {
        let begin_info = if let Some(inheritance) = inheritance {
            CommandBufferBeginInfo::builder().inheritance_info(inheritance)
        } else {
            CommandBufferBeginInfo::builder()
        }
        .flags(flags);
        unsafe {
            self.device
                .inner
                .begin_command_buffer(self.handle, &begin_info)
        }
    }

    pub fn begin_render_pass(
        &self,
        render_pass: &RenderPass,
        framebuffer: &Framebuffer,
        render_area: Rect2D,
        clear_values: &[ClearValue],
        contents: SubpassContents,
    ) {
        let create_info = RenderPassBeginInfo::builder()
            .render_pass(render_pass.handle)
            .framebuffer(framebuffer.handle)
            .render_area(render_area)
            .clear_values(clear_values);
        unsafe {
            self.device
                .inner
                .cmd_begin_render_pass(self.handle, &create_info, contents)
        }
    }

    pub fn next_subpass(&self, contents: SubpassContents) {
        unsafe { self.device.inner.cmd_next_subpass(self.handle, contents) }
    }

    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.inner.cmd_draw(
                self.handle,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            )
        };
    }

    pub fn end_render_pass(&self) {
        unsafe { self.device.inner.cmd_end_render_pass(self.handle) }
    }

    pub fn bind_pipeline(&self, binding_point: PipelineBindPoint, pipeline: &Pipeline) {
        unsafe {
            self.device
                .inner
                .cmd_bind_pipeline(self.handle, binding_point, pipeline.handle)
        }
    }

    pub fn end(&self) -> VkResult<()> {
        unsafe { self.device.inner.end_command_buffer(self.handle) }
    }

    pub fn pipeline_barrier(
        &self,
        src_stage_mask: PipelineStageFlags,
        dst_stage_mask: PipelineStageFlags,
        dependency_flags: DependencyFlags,
        memory_barriers: &[MemoryBarrier],
        buffer_memory_barriers: &[BufferMemoryBarrier],
        image_memory_barriers: &[ImageMemoryBarrier],
    ) {
        unsafe {
            self.device.inner.cmd_pipeline_barrier(
                self.handle,
                src_stage_mask,
                dst_stage_mask,
                dependency_flags,
                memory_barriers,
                buffer_memory_barriers,
                image_memory_barriers,
            )
        }
    }
}

impl PipelineCache {
    pub fn create_graphics(
        &self,
        create_infos: &[GraphicsPipelineCreateInfo],
    ) -> VkResult<Vec<Pipeline>> {
        let handles = match unsafe {
            self.device
                .inner
                .create_graphics_pipelines(self.handle, create_infos, None)
        } {
            Ok(inner) => inner,
            Err((pipelines, err)) => {
                for pipeline in pipelines {
                    if pipeline != vk::Pipeline::null() {
                        unsafe { self.device.inner.destroy_pipeline(pipeline, None) };
                    }
                }
                return Err(err);
            }
        };
        Ok(handles
            .into_iter()
            .map(|handle| Pipeline {
                handle,
                device: self.device.clone(),
            })
            .collect())
    }
}

impl RenderPass {
    pub fn create_framebuffer(
        &self,
        flags: FramebufferCreateFlags,
        attachments: &[&ImageView],
        width: u32,
        height: u32,
        layers: u32,
    ) -> VkResult<Framebuffer> {
        let attachments: Vec<_> = attachments
            .iter()
            .map(|attachment| attachment.handle)
            .collect();
        let create_info = FramebufferCreateInfo::builder()
            .flags(flags)
            .render_pass(self.handle)
            .attachments(&attachments)
            .width(width)
            .height(height)
            .layers(layers);
        let handle = unsafe { self.device.inner.create_framebuffer(&create_info, None) }?;
        Ok(Framebuffer {
            handle,
            device: self.device.clone(),
        })
    }
}
