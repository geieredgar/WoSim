use std::{mem::size_of, sync::Arc};

use eyre::Context as EyreContext;
use vulkan::{
    AccessFlags, BufferCopy, BufferMemoryBarrier, ClearColorValue, ClearDepthStencilValue,
    ClearValue, CommandBuffer, CommandBufferLevel, CommandBufferUsageFlags, CommandPool,
    CommandPoolCreateFlags, CommandPoolResetFlags, DependencyFlags, DescriptorPoolSetup, Device,
    DrawIndexedIndirectCommand, Fence, FenceCreateFlags, Offset2D, PipelineBindPoint,
    PipelineStageFlags, QueryPipelineStatisticFlags, QueryPool, QueryResultFlags, QueryType,
    Rect2D, Semaphore, SubmitInfo, SubpassContents,
};

use crate::{
    context::Context,
    cull::CullFrame,
    egui::EguiFrame,
    renderer::{RenderError, RenderResult, RenderTimestamps},
    scene::SceneFrame,
    view::View,
};

pub struct Frame {
    cull: CullFrame,
    scene: SceneFrame,
    egui: EguiFrame,
    command_buffer: CommandBuffer,
    main_queue_fence: Fence,
    image_ready: Semaphore,
    render_finished: Semaphore,
    timestamp_pool: QueryPool,
    command_pool: CommandPool,
}

impl Frame {
    pub fn new(device: &Arc<Device>, context: &Context) -> eyre::Result<Self> {
        let command_pool = device.create_command_pool(
            CommandPoolCreateFlags::TRANSIENT,
            device.main_queue_family_index(),
        )?;
        let timestamp_pool = device.create_query_pool(
            QueryType::TIMESTAMP,
            2,
            QueryPipelineStatisticFlags::empty(),
        )?;
        let mut command_buffers = command_pool.allocate(CommandBufferLevel::PRIMARY, 1)?;
        let command_buffer = command_buffers.remove(0);
        let main_queue_fence = device.create_fence(FenceCreateFlags::SIGNALED)?;
        let image_ready = device.create_semaphore()?;
        let render_finished = device.create_semaphore()?;
        let egui = EguiFrame::new(device, &context.egui, &context.descriptor_pool)
            .wrap_err("could not create egui frame data")?;
        let scene = SceneFrame::new(
            device,
            &context.scene,
            2usize.pow(20),
            &context.descriptor_pool,
        )
        .wrap_err("could not create scene frame data")?;
        let cull = CullFrame::new(
            device,
            &context.cull,
            &context.scene,
            &scene,
            &context.descriptor_pool,
        )
        .wrap_err("could not create cull frame data")?;
        Ok(Self {
            cull,
            scene,
            egui,
            command_buffer,
            main_queue_fence,
            image_ready,
            render_finished,
            timestamp_pool,
            command_pool,
        })
    }

    pub fn setup_view(&mut self, device: &Arc<Device>, view: &View) {
        self.cull.setup_view(device, &view.depth);
    }

    pub fn render(
        &mut self,
        device: &Arc<Device>,
        context: &mut Context,
        view: &View,
    ) -> Result<RenderResult, RenderError> {
        self.main_queue_fence.wait()?;
        let last_draw_count = self
            .scene
            .update(&context.scene, view.swapchain.image_extent())
            .wrap_err("could not update scene frame data")?;
        self.main_queue_fence.reset()?;
        let timestamps: Option<Vec<u64>> =
            self.timestamp_pool
                .results(0, 2, QueryResultFlags::TYPE_64)?;
        self.command_pool.reset(CommandPoolResetFlags::empty())?;
        self.command_buffer
            .begin(CommandBufferUsageFlags::ONE_TIME_SUBMIT, None)?;
        self.command_buffer
            .reset_query_pool(&self.timestamp_pool, 0, 2);
        self.command_buffer.write_timestamp(
            PipelineStageFlags::TOP_OF_PIPE,
            &self.timestamp_pool,
            0,
        );
        self.egui
            .prepare(device, &self.command_buffer, &mut context.egui)
            .wrap_err("could not prepare egui rendering")?;
        self.command_buffer
            .fill_buffer(&self.scene.draw_count, 0, size_of::<u32>() as u64, 0);
        let buffer_memory_barriers = [BufferMemoryBarrier::builder()
            .src_access_mask(AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(AccessFlags::SHADER_READ | AccessFlags::SHADER_WRITE)
            .src_queue_family_index(0)
            .dst_queue_family_index(0)
            .buffer(*self.scene.draw_count)
            .offset(0)
            .size(size_of::<u32>() as u64)
            .build()];
        self.command_buffer.pipeline_barrier(
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::COMPUTE_SHADER,
            DependencyFlags::empty(),
            &[],
            &buffer_memory_barriers,
            &[],
        );
        self.command_buffer
            .bind_pipeline(PipelineBindPoint::COMPUTE, &context.cull.pipeline);
        self.command_buffer.bind_descriptor_sets(
            PipelineBindPoint::COMPUTE,
            &context.cull.pipeline_layout,
            0,
            &[&self.cull.descriptor_set],
            &[],
        );
        self.command_buffer
            .dispatch((self.scene.objects.len() as u32 + 255) / 256, 1, 1);
        let buffer_memory_barriers = [
            BufferMemoryBarrier::builder()
                .src_access_mask(AccessFlags::SHADER_WRITE)
                .dst_access_mask(AccessFlags::INDIRECT_COMMAND_READ)
                .src_queue_family_index(0)
                .dst_queue_family_index(0)
                .buffer(*self.scene.commands)
                .offset(0)
                .size((self.scene.objects.len() * size_of::<DrawIndexedIndirectCommand>()) as u64)
                .build(),
            BufferMemoryBarrier::builder()
                .src_access_mask(AccessFlags::SHADER_WRITE)
                .dst_access_mask(AccessFlags::TRANSFER_READ)
                .src_queue_family_index(0)
                .dst_queue_family_index(0)
                .buffer(*self.scene.draw_count)
                .offset(0)
                .size(size_of::<u32>() as u64)
                .build(),
        ];
        self.command_buffer.pipeline_barrier(
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::DRAW_INDIRECT | PipelineStageFlags::TRANSFER,
            DependencyFlags::empty(),
            &[],
            &buffer_memory_barriers,
            &[],
        );
        let regions = [BufferCopy::builder()
            .src_offset(0)
            .dst_offset(0)
            .size(size_of::<u32>() as u64)
            .build()];
        self.command_buffer.copy_buffer(
            &self.scene.draw_count,
            self.scene.draw_count_read_back.buffer(),
            &regions,
        );
        let (image_index, suboptimal) = view.swapchain.acquire_next_image(&self.image_ready)?;
        let clear_values = [
            ClearValue {
                color: ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            ClearValue {
                depth_stencil: ClearDepthStencilValue {
                    depth: 0f32,
                    stencil: 0,
                },
            },
            ClearValue::default(),
        ];
        self.command_buffer.begin_render_pass(
            &view.render_pass,
            &view.framebuffers[image_index as usize],
            Rect2D {
                offset: Offset2D { x: 0, y: 0 },
                extent: view.swapchain.image_extent(),
            },
            &clear_values,
            SubpassContents::INLINE,
        );
        self.scene.render(
            &self.command_buffer,
            &context.scene,
            &view.scene,
            context.configuration.use_draw_count,
        );
        self.command_buffer.next_subpass(SubpassContents::INLINE);
        self.egui
            .render(
                &self.command_buffer,
                &view.egui,
                &mut context.egui,
                view.swapchain.image_extent(),
            )
            .wrap_err("could not render egui")?;
        self.command_buffer.end_render_pass();
        self.command_buffer.write_timestamp(
            PipelineStageFlags::BOTTOM_OF_PIPE,
            &self.timestamp_pool,
            1,
        );
        self.command_buffer.end()?;
        let command_buffers = [*self.command_buffer];
        let signal_semaphores = [*self.render_finished, *view.depth.ready];
        let wait_semaphores = [*self.image_ready, *view.depth.ready];
        let wait_dst_stage_mask = [
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::COMPUTE_SHADER,
        ];
        let submits = [SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .signal_semaphores(&signal_semaphores)
            .build()];
        device.submit(&submits, &self.main_queue_fence)?;
        let suboptimal = view.swapchain.present(image_index, &self.render_finished)? || suboptimal;
        let timestamps = if let Some(timestamps) = timestamps {
            Some(RenderTimestamps {
                begin: timestamps[0] as f64 * context.configuration.timestamp_period,
                end: timestamps[1] as f64 * context.configuration.timestamp_period,
            })
        } else {
            None
        };
        Ok(RenderResult {
            suboptimal,
            timestamps,
            last_draw_count,
        })
    }

    pub fn pool_setup() -> DescriptorPoolSetup {
        EguiFrame::pool_setup() + CullFrame::pool_setup() + SceneFrame::pool_setup()
    }
}
