use std::{ops::Deref, sync::Arc};

use ash::{
    prelude::VkResult,
    version::DeviceV1_0,
    vk::{
        self, BufferMemoryBarrier, CommandBufferAllocateInfo, CommandBufferBeginInfo,
        CommandBufferInheritanceInfo, CommandBufferLevel, CommandBufferUsageFlags,
        CommandPoolResetFlags, DependencyFlags, ImageMemoryBarrier, MemoryBarrier,
        PipelineStageFlags,
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

impl Object<vk::CommandPool> {
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

impl Object<vk::Fence> {
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

impl Object<vk::CommandBuffer> {
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
