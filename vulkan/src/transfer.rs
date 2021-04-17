use std::sync::{Arc, Mutex};

use ash::{
    prelude::VkResult,
    vk::{
        AccessFlags, BufferImageCopy, CommandBufferUsageFlags, CommandPoolCreateFlags,
        CommandPoolResetFlags, DependencyFlags, FenceCreateFlags, ImageLayout, ImageMemoryBarrier,
        ImageSubresourceRange, PipelineStageFlags, SubmitInfo,
    },
};

use super::{Buffer, CommandBuffer, CommandPool, Device, Fence, Image, Semaphore};

pub struct TransferPool {
    buffers: Arc<Mutex<Vec<TransferBufferInner>>>,
    device: Arc<Device>,
}

impl TransferPool {
    pub(super) fn new(device: Arc<Device>) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(Vec::new())),
            device,
        }
    }

    pub fn allocate(&mut self) -> VkResult<TransferBuffer> {
        let inner = Some(if let Some(inner) = self.buffers.lock().unwrap().pop() {
            inner
        } else {
            TransferBufferInner::new(&self.device)?
        });
        Ok(TransferBuffer {
            inner,
            buffers: self.buffers.clone(),
            device: self.device.clone(),
        })
    }
}

pub struct TransferBuffer {
    inner: Option<TransferBufferInner>,
    buffers: Arc<Mutex<Vec<TransferBufferInner>>>,
    device: Arc<Device>,
}

impl TransferBuffer {
    pub fn submit(&self) -> VkResult<()> {
        let inner = self.inner.as_ref().unwrap();
        let (fence, semaphores) = if let Some(destination) = inner.destination.as_ref() {
            destination.command_buffer.end()?;
            let command_buffers = [destination.command_buffer.handle];
            let wait_semaphores = [destination.semaphore.handle];
            let wait_dst_stage_mask = [PipelineStageFlags::BOTTOM_OF_PIPE];
            let submits = [SubmitInfo::builder()
                .command_buffers(&command_buffers)
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_dst_stage_mask)
                .build()];
            self.device.submit(&submits, &inner.fence)?;
            (Some(&inner.fence), vec![destination.semaphore.handle])
        } else {
            (None, Vec::new())
        };
        inner.command_buffer.end()?;
        let command_buffers = [inner.command_buffer.handle];
        let submits = [SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .signal_semaphores(&semaphores)
            .build()];
        self.device.transfer_submit(&submits, fence)?;
        Ok(())
    }

    pub fn transfer_buffer_to_image(
        &mut self,
        src: &Buffer,
        dst: &Image,
        initial_layout: ImageLayout,
        final_layout: ImageLayout,
        subresource_range: ImageSubresourceRange,
        regions: &[BufferImageCopy],
    ) {
        let inner = self.inner.as_ref().unwrap();
        inner.command_buffer.pipeline_barrier(
            PipelineStageFlags::HOST,
            PipelineStageFlags::TRANSFER,
            DependencyFlags::empty(),
            &[],
            &[],
            &[ImageMemoryBarrier::builder()
                .image(dst.handle)
                .src_access_mask(AccessFlags::empty())
                .dst_access_mask(AccessFlags::TRANSFER_WRITE)
                .old_layout(initial_layout)
                .new_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
                .subresource_range(subresource_range)
                .build()],
        );
        inner.command_buffer.copy_buffer_to_image(
            src,
            dst,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            regions,
        );
        inner.image_pipeline_barrier_with_ownership_transfer(
            &self.device,
            ImageTransferInfo {
                src_access_mask: AccessFlags::TRANSFER_WRITE,
                src_stage_mask: PipelineStageFlags::TRANSFER,
                dependency_flags: DependencyFlags::empty(),
                dst_stage_mask: PipelineStageFlags::BOTTOM_OF_PIPE,
                dst_access_mask: AccessFlags::empty(),
                initial_layout: ImageLayout::TRANSFER_DST_OPTIMAL,
                final_layout,
            },
            dst,
            subresource_range,
        );
    }

    pub fn ready(&self) -> VkResult<bool> {
        self.inner.as_ref().unwrap().fence.status()
    }
}

impl Drop for TransferBuffer {
    fn drop(&mut self) {
        let inner = self.inner.take().unwrap();
        inner.fence.wait().unwrap();
        inner
            .command_pool
            .reset(CommandPoolResetFlags::empty())
            .unwrap();
        if let Some(destination) = inner.destination.as_ref() {
            destination
                .command_pool
                .reset(CommandPoolResetFlags::empty())
                .unwrap();
        }
        inner.fence.reset().unwrap();
        self.buffers.lock().unwrap().push(inner)
    }
}

struct TransferBufferInner {
    command_buffer: CommandBuffer,
    command_pool: CommandPool,
    fence: Fence,
    destination: Option<TransferDestination>,
}

struct TransferDestination {
    command_pool: CommandPool,
    command_buffer: CommandBuffer,
    semaphore: Semaphore,
}

impl TransferBufferInner {
    fn new(device: &Arc<Device>) -> VkResult<Self> {
        let command_pool = device.create_command_pool(
            CommandPoolCreateFlags::TRANSIENT,
            device.transfer_queue_family_index(),
        )?;
        let command_buffer = command_pool.allocate_single_primary()?;
        command_buffer.begin(CommandBufferUsageFlags::ONE_TIME_SUBMIT, None)?;
        Ok(Self {
            command_buffer,
            command_pool,
            fence: device.create_fence(FenceCreateFlags::empty())?,
            destination: TransferDestination::new(device)?,
        })
    }

    fn image_pipeline_barrier_with_ownership_transfer(
        &self,
        device: &Device,
        info: ImageTransferInfo,
        image: &Image,
        subresource_range: ImageSubresourceRange,
    ) {
        if let Some(destination) = self.destination.as_ref() {
            self.command_buffer.pipeline_barrier(
                info.src_stage_mask,
                PipelineStageFlags::BOTTOM_OF_PIPE,
                info.dependency_flags,
                &[],
                &[],
                &[ImageMemoryBarrier::builder()
                    .src_access_mask(info.src_access_mask)
                    .src_queue_family_index(device.main_queue_family_index())
                    .dst_queue_family_index(device.transfer_queue_family_index())
                    .old_layout(info.initial_layout)
                    .new_layout(info.final_layout)
                    .image(image.handle)
                    .subresource_range(subresource_range)
                    .build()],
            );
            destination.command_buffer.pipeline_barrier(
                PipelineStageFlags::TOP_OF_PIPE,
                info.dst_stage_mask,
                info.dependency_flags,
                &[],
                &[],
                &[ImageMemoryBarrier::builder()
                    .dst_access_mask(info.dst_access_mask)
                    .src_queue_family_index(device.main_queue_family_index())
                    .dst_queue_family_index(device.transfer_queue_family_index())
                    .old_layout(info.initial_layout)
                    .new_layout(info.final_layout)
                    .image(image.handle)
                    .subresource_range(subresource_range)
                    .build()],
            );
        } else {
            self.command_buffer.pipeline_barrier(
                info.src_stage_mask,
                info.dst_stage_mask,
                info.dependency_flags,
                &[],
                &[],
                &[ImageMemoryBarrier::builder()
                    .src_access_mask(info.src_access_mask)
                    .dst_access_mask(info.dst_access_mask)
                    .old_layout(info.initial_layout)
                    .new_layout(info.final_layout)
                    .image(image.handle)
                    .subresource_range(subresource_range)
                    .build()],
            )
        }
    }
}

impl TransferDestination {
    fn new(device: &Arc<Device>) -> VkResult<Option<Self>> {
        Ok(if device.has_dedicated_transfer_queue() {
            let command_pool = device.create_command_pool(
                CommandPoolCreateFlags::TRANSIENT,
                device.main_queue_family_index(),
            )?;
            let command_buffer = command_pool.allocate_single_primary()?;
            Some(Self {
                command_buffer,
                command_pool,
                semaphore: device.create_semaphore()?,
            })
        } else {
            None
        })
    }
}

pub struct ImageTransferInfo {
    pub src_stage_mask: PipelineStageFlags,
    pub src_access_mask: AccessFlags,
    pub dependency_flags: DependencyFlags,
    pub dst_stage_mask: PipelineStageFlags,
    pub dst_access_mask: AccessFlags,
    pub initial_layout: ImageLayout,
    pub final_layout: ImageLayout,
}
