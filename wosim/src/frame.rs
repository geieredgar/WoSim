use std::sync::Arc;

use wosim_common::vulkan::{
    ApiResult, ClearColorValue, ClearValue, CommandBuffer, CommandBufferLevel,
    CommandBufferUsageFlags, CommandPool, CommandPoolCreateFlags, CommandPoolResetFlags,
    DescriptorPoolSetup, Device, Fence, FenceCreateFlags, Framebuffer, FramebufferCreateFlags,
    Offset2D, PipelineBindPoint, PipelineStageFlags, Rect2D, Semaphore, SubmitInfo,
    SubpassContents,
};

use crate::{context::Context, egui::EguiFrame, error::Error, renderer::RenderResult, view::View};

pub struct Frame {
    egui: EguiFrame,
    command_buffer: CommandBuffer,
    main_queue_fence: Fence,
    image_ready: Semaphore,
    render_finished: Semaphore,
    command_pool: CommandPool,
    framebuffers: Vec<Framebuffer>,
}

impl Frame {
    pub fn new(device: &Arc<Device>, context: &Context, view: &View) -> Result<Self, Error> {
        let command_pool = device.create_command_pool(
            CommandPoolCreateFlags::TRANSIENT,
            device.main_queue_family_index(),
        )?;
        let mut command_buffers = command_pool.allocate(CommandBufferLevel::PRIMARY, 1)?;
        let command_buffer = command_buffers.remove(0);
        let main_queue_fence = device.create_fence(FenceCreateFlags::SIGNALED)?;
        let image_ready = device.create_semaphore()?;
        let render_finished = device.create_semaphore()?;
        let image_extent = view.swapchain.image_extent();
        let framebuffers: Result<_, ApiResult> = view
            .images
            .iter()
            .map(|image| {
                let attachments = [image.view()];
                view.render_pass.create_framebuffer(
                    FramebufferCreateFlags::empty(),
                    &attachments,
                    image_extent.width,
                    image_extent.height,
                    1,
                )
            })
            .collect();
        let framebuffers = framebuffers?;
        let egui = EguiFrame::new(device, &context.egui, &view.descriptor_pool)?;
        Ok(Self {
            egui,
            command_buffer,
            main_queue_fence,
            image_ready,
            render_finished,
            command_pool,
            framebuffers,
        })
    }

    pub fn render(
        &mut self,
        device: &Arc<Device>,
        context: &mut Context,
        view: &View,
    ) -> Result<RenderResult, Error> {
        self.main_queue_fence.wait()?;
        self.main_queue_fence.reset()?;
        self.command_pool.reset(CommandPoolResetFlags::empty())?;
        self.command_buffer
            .begin(CommandBufferUsageFlags::ONE_TIME_SUBMIT, None)?;
        self.egui
            .prepare(device, &self.command_buffer, &mut context.egui)?;
        let (image_index, suboptimal) = view.swapchain.acquire_next_image(&self.image_ready)?;
        let clear_values = [ClearValue {
            color: ClearColorValue {
                float32: [0.0, 1.0, 0.0, 1.0],
            },
        }];
        self.command_buffer.begin_render_pass(
            &view.render_pass,
            &self.framebuffers[image_index as usize],
            Rect2D {
                offset: Offset2D { x: 0, y: 0 },
                extent: view.swapchain.image_extent(),
            },
            &clear_values,
            SubpassContents::INLINE,
        );
        self.command_buffer
            .bind_pipeline(PipelineBindPoint::GRAPHICS, &view.pipeline);
        self.command_buffer.draw(3, 1, 0, 0);
        self.egui.render(
            &self.command_buffer,
            &view.egui,
            &mut context.egui,
            view.swapchain.image_extent(),
        )?;
        self.command_buffer.end_render_pass();
        self.command_buffer.end()?;
        let command_buffers = [*self.command_buffer];
        let signal_semaphores = [*self.render_finished];
        let wait_semaphores = [*self.image_ready];
        let wait_dst_stage_mask = [PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
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

    pub fn pool_setup() -> DescriptorPoolSetup {
        EguiFrame::pool_setup()
    }
}
