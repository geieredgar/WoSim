use std::sync::Arc;

use vulkan::{
    mip_levels_for_extent, AccessFlags, ApiResult, AttachmentDescription, AttachmentLoadOp,
    AttachmentReference, AttachmentStoreOp, ClearColorValue, CommandBufferUsageFlags,
    CommandPoolResetFlags, DependencyFlags, Device, Framebuffer, FramebufferCreateFlags,
    ImageAspectFlags, ImageLayout, ImageMemoryBarrier, ImageSubresourceRange, PipelineBindPoint,
    PipelineStageFlags, RenderPass, RenderPassCreateInfo, SampleCountFlags, SubmitInfo,
    SubpassDependency, SubpassDescription, Swapchain, SwapchainImage, SUBPASS_EXTERNAL,
};

use crate::{context::Context, depth::DepthView, egui::EguiView, error::Error, scene::SceneView};

pub struct View {
    pub depth: DepthView,
    pub egui: EguiView,
    pub scene: SceneView,
    pub framebuffers: Vec<Framebuffer>,
    pub render_pass: RenderPass,
    pub images: Vec<SwapchainImage>,
    pub swapchain: Arc<Swapchain>,
}

impl View {
    pub fn new(
        device: &Arc<Device>,
        context: &Context,
        swapchain: Arc<Swapchain>,
    ) -> Result<Self, Error> {
        let image_format = swapchain.image_format();
        let attachments = [
            AttachmentDescription::builder()
                .format(swapchain.image_format())
                .samples(SampleCountFlags::TYPE_1)
                .load_op(AttachmentLoadOp::CLEAR)
                .store_op(AttachmentStoreOp::STORE)
                .stencil_load_op(AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(AttachmentStoreOp::DONT_CARE)
                .initial_layout(ImageLayout::UNDEFINED)
                .final_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .build(),
            AttachmentDescription::builder()
                .format(context.configuration.depth_format)
                .samples(SampleCountFlags::TYPE_1)
                .load_op(AttachmentLoadOp::CLEAR)
                .store_op(AttachmentStoreOp::STORE)
                .stencil_load_op(AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(AttachmentStoreOp::DONT_CARE)
                .initial_layout(ImageLayout::UNDEFINED)
                .final_layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .build(),
            AttachmentDescription::builder()
                .format(context.configuration.depth_format)
                .samples(SampleCountFlags::TYPE_1)
                .load_op(AttachmentLoadOp::LOAD)
                .store_op(AttachmentStoreOp::STORE)
                .stencil_load_op(AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(AttachmentStoreOp::DONT_CARE)
                .initial_layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
                .final_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .build(),
            AttachmentDescription::builder()
                .format(swapchain.image_format())
                .samples(SampleCountFlags::TYPE_1)
                .load_op(AttachmentLoadOp::LOAD)
                .store_op(AttachmentStoreOp::STORE)
                .stencil_load_op(AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(AttachmentStoreOp::DONT_CARE)
                .initial_layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .final_layout(ImageLayout::PRESENT_SRC_KHR)
                .build(),
        ];
        let color_attachments = [AttachmentReference::builder()
            .attachment(0)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];
        let post_color_attachments = [AttachmentReference::builder()
            .attachment(3)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];
        let pre_pass_depth_stencil_attachment = AttachmentReference::builder()
            .attachment(1)
            .layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        let depth_stencil_attachment = AttachmentReference::builder()
            .attachment(2)
            .layout(ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
        let subpasses = [
            SubpassDescription::builder()
                .color_attachments(&[])
                .depth_stencil_attachment(&pre_pass_depth_stencil_attachment)
                .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
                .build(),
            SubpassDescription::builder()
                .color_attachments(&color_attachments)
                .depth_stencil_attachment(&depth_stencil_attachment)
                .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
                .build(),
            SubpassDescription::builder()
                .color_attachments(&post_color_attachments)
                .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
                .build(),
        ];
        let dependencies = [
            SubpassDependency::builder()
                .src_subpass(SUBPASS_EXTERNAL)
                .dst_subpass(0)
                .src_stage_mask(PipelineStageFlags::TOP_OF_PIPE)
                .dst_stage_mask(PipelineStageFlags::EARLY_FRAGMENT_TESTS)
                .src_access_mask(AccessFlags::empty())
                .dst_access_mask(AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .build(),
            SubpassDependency::builder()
                .src_subpass(0)
                .dst_subpass(1)
                .src_stage_mask(PipelineStageFlags::LATE_FRAGMENT_TESTS)
                .dst_stage_mask(PipelineStageFlags::EARLY_FRAGMENT_TESTS)
                .src_access_mask(AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .dst_access_mask(AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ)
                .build(),
            SubpassDependency::builder()
                .src_subpass(1)
                .dst_subpass(2)
                .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(AccessFlags::COLOR_ATTACHMENT_WRITE)
                .build(),
        ];
        let create_info = RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        let render_pass = device.create_render_pass(&create_info)?;
        let images = swapchain.images()?;
        let image_extent = swapchain.image_extent();
        let depth_pyramid_mip_levels = mip_levels_for_extent(image_extent);
        let egui = EguiView::new(
            device,
            &context.egui,
            &context.pipeline_cache,
            image_extent,
            image_format,
            &render_pass,
            2,
        )?;
        let scene = SceneView::new(
            &context.scene,
            &render_pass,
            &context.pipeline_cache,
            0,
            image_extent,
        )?;
        let depth = DepthView::new(
            device,
            &context.depth,
            &context.configuration,
            image_extent,
            depth_pyramid_mip_levels,
        )?;
        let framebuffers: Result<_, ApiResult> = images
            .iter()
            .map(|image| {
                render_pass.create_framebuffer(
                    FramebufferCreateFlags::empty(),
                    &[
                        image.view(),
                        &depth.image_view,
                        &depth.image_view,
                        image.view(),
                    ],
                    image_extent.width,
                    image_extent.height,
                    1,
                )
            })
            .collect();
        let framebuffers = framebuffers?;
        Ok(Self {
            depth,
            egui,
            scene,
            framebuffers,
            render_pass,
            images,
            swapchain,
        })
    }

    pub fn setup(&mut self, device: &Arc<Device>, context: &mut Context) -> Result<(), ApiResult> {
        context.command_pool.reset(CommandPoolResetFlags::empty())?;
        context
            .command_buffer
            .begin(CommandBufferUsageFlags::ONE_TIME_SUBMIT, None)?;
        let subresource_range = ImageSubresourceRange::builder()
            .aspect_mask(ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(self.depth.pyramid_views.len() as u32)
            .base_array_layer(0)
            .layer_count(1)
            .build();
        let image_memory_barriers = [ImageMemoryBarrier::builder()
            .image(*self.depth.pyramid_image)
            .src_access_mask(AccessFlags::empty())
            .dst_access_mask(AccessFlags::TRANSFER_WRITE)
            .old_layout(ImageLayout::UNDEFINED)
            .new_layout(ImageLayout::GENERAL)
            .subresource_range(subresource_range)
            .build()];
        context.command_buffer.pipeline_barrier(
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::TRANSFER,
            DependencyFlags::empty(),
            &[],
            &[],
            &image_memory_barriers,
        );
        context.command_buffer.clear_color_image(
            &self.depth.pyramid_image,
            ImageLayout::GENERAL,
            &ClearColorValue {
                float32: [0.0, 0.0, 0.0, 0.0],
            },
            &[subresource_range],
        );
        context.command_buffer.end()?;
        let command_buffers = [*context.command_buffer];
        let signal_semaphores = [*self.depth.ready];
        let wait_semaphores = [];
        let wait_dst_stage_mask = [];
        let submits = [SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_dst_stage_mask)
            .signal_semaphores(&signal_semaphores)
            .build()];
        device.submit_without_fence(&submits)?;
        Ok(())
    }

    pub const MAX_MIP_LEVELS: usize = 13;
}
