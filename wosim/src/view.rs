use std::sync::Arc;

use wosim_common::vulkan::{
    mip_levels_for_extent, AccessFlags, AttachmentDescription, AttachmentLoadOp,
    AttachmentReference, AttachmentStoreOp, DescriptorPool, Device, ImageLayout, PipelineBindPoint,
    PipelineStageFlags, RenderPass, RenderPassCreateInfo, SampleCountFlags, SubpassDependency,
    SubpassDescription, Swapchain, SwapchainImage, SUBPASS_EXTERNAL,
};

use crate::{
    context::Context, depth::DepthView, egui::EguiView, error::Error, frame::Frame,
    scene::SceneView,
};

pub struct View {
    pub depth: DepthView,
    pub egui: EguiView,
    pub scene: SceneView,
    pub descriptor_pool: DescriptorPool,
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
        let descriptor_pool =
            (Frame::pool_setup(depth_pyramid_mip_levels) * 2).create_pool(device)?;
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
        let depth = DepthView::new(device, depth_pyramid_mip_levels)?;
        Ok(Self {
            depth,
            scene,
            egui,
            descriptor_pool,
            render_pass,
            images,
            swapchain,
        })
    }
}
