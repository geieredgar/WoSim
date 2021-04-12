use std::sync::Arc;

use wosim_common::vulkan::{
    AccessFlags, CommandBuffer, CommandBufferLevel, CommandBufferUsageFlags, CommandPool,
    CommandPoolCreateFlags, CommandPoolResetFlags, DependencyFlags, Device, Fence,
    FenceCreateFlags, ImageAspectFlags, ImageLayout, ImageMemoryBarrier, ImageSubresourceRange,
    PipelineStageFlags, Semaphore, SubmitInfo,
};

use crate::{context::Context, error::Error, renderer::RenderResult, view::View};

pub struct Frame {
    command_buffer: CommandBuffer,
    main_queue_fence: Fence,
    image_ready: Semaphore,
    render_finished: Semaphore,
    command_pool: CommandPool,
}

impl Frame {
    pub fn new(device: &Arc<Device>, _context: &Context, _view: &View) -> Result<Self, Error> {
        let command_pool = device.create_command_pool(
            CommandPoolCreateFlags::TRANSIENT,
            device.main_queue_family_index(),
        )?;
        let mut command_buffers = command_pool.allocate(CommandBufferLevel::PRIMARY, 1)?;
        let command_buffer = command_buffers.remove(0);
        let main_queue_fence = device.create_fence(FenceCreateFlags::SIGNALED)?;
        let image_ready = device.create_semaphore()?;
        let render_finished = device.create_semaphore()?;
        Ok(Self {
            command_buffer,
            command_pool,
            main_queue_fence,
            image_ready,
            render_finished,
        })
    }

    pub fn render(
        &mut self,
        device: &Arc<Device>,
        _context: &Context,
        view: &View,
    ) -> Result<RenderResult, Error> {
        self.main_queue_fence.wait()?;
        self.main_queue_fence.reset()?;
        self.command_pool.reset(CommandPoolResetFlags::empty())?;
        self.command_buffer
            .begin(CommandBufferUsageFlags::ONE_TIME_SUBMIT, None)?;
        let (image_index, suboptimal) = view.swapchain.acquire_next_image(&self.image_ready)?;
        let image_memory_barriers = [ImageMemoryBarrier::builder()
            .image(*view.images[image_index as usize])
            .old_layout(ImageLayout::UNDEFINED)
            .new_layout(ImageLayout::PRESENT_SRC_KHR)
            .src_access_mask(AccessFlags::empty())
            .dst_access_mask(AccessFlags::empty())
            .subresource_range(
                ImageSubresourceRange::builder()
                    .level_count(1)
                    .layer_count(1)
                    .aspect_mask(ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .base_mip_level(0)
                    .build(),
            )
            .build()];
        self.command_buffer.pipeline_barrier(
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::BOTTOM_OF_PIPE,
            DependencyFlags::empty(),
            &[],
            &[],
            &image_memory_barriers,
        );
        self.command_buffer.end()?;
        let command_buffers = [*self.command_buffer];
        let signal_semaphores = [*self.render_finished];
        let wait_semaphores = [*self.image_ready];
        let wait_dst_stage_mask = [PipelineStageFlags::BOTTOM_OF_PIPE];
        let submits = [SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .signal_semaphores(&signal_semaphores)
            .build()];
        device.submit(&submits, &self.main_queue_fence)?;
        let suboptimal = view.swapchain.present(image_index, &self.render_finished)? || suboptimal;
        Ok(RenderResult { suboptimal })
    }
}
