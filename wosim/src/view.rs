use std::{ffi::CString, sync::Arc};

use wosim_common::vulkan::{
    AccessFlags, AttachmentDescription, AttachmentLoadOp, AttachmentReference, AttachmentStoreOp,
    BlendFactor, BlendOp, ColorComponentFlags, CompareOp, CullModeFlags, Device, FrontFace,
    GraphicsPipelineCreateInfo, ImageLayout, LogicOp, Offset2D, Pipeline, PipelineBindPoint,
    PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineDepthStencilStateCreateInfo, PipelineInputAssemblyStateCreateInfo,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
    PipelineShaderStageCreateInfo, PipelineStageFlags, PipelineVertexInputStateCreateInfo,
    PipelineViewportStateCreateInfo, PolygonMode, PrimitiveTopology, Rect2D, RenderPass,
    RenderPassCreateInfo, SampleCountFlags, ShaderStageFlags, SubpassDependency,
    SubpassDescription, Swapchain, SwapchainImage, Viewport, SUBPASS_EXTERNAL,
};

use crate::{context::Context, error::Error};

pub struct View {
    pub pipeline: Pipeline,
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
        let attachments = [AttachmentDescription::builder()
            .format(image_format)
            .samples(SampleCountFlags::TYPE_1)
            .load_op(AttachmentLoadOp::CLEAR)
            .store_op(AttachmentStoreOp::STORE)
            .stencil_load_op(AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(AttachmentStoreOp::DONT_CARE)
            .initial_layout(ImageLayout::UNDEFINED)
            .final_layout(ImageLayout::PRESENT_SRC_KHR)
            .build()];
        let color_attachments = [AttachmentReference::builder()
            .attachment(0)
            .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build()];
        let subpasses = [SubpassDescription::builder()
            .color_attachments(&color_attachments)
            .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
            .build()];
        let dependencies = [SubpassDependency::builder()
            .src_subpass(SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(AccessFlags::empty())
            .dst_access_mask(AccessFlags::COLOR_ATTACHMENT_WRITE)
            .build()];
        let create_info = RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);
        let render_pass = device.create_render_pass(&create_info)?;
        let binding_descriptions = [];
        let attribute_descriptions = [];
        let vertex_input_state = PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);
        let input_assembly_state = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let image_extent = swapchain.image_extent();
        let viewports = [Viewport {
            x: 0f32,
            y: 0f32,
            width: image_extent.width as f32,
            height: image_extent.height as f32,
            min_depth: 0f32,
            max_depth: 1f32,
        }];
        let scissors = [Rect2D {
            offset: Offset2D { x: 0, y: 0 },
            extent: image_extent,
        }];
        let viewport_state = PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);
        let rasterization_state = PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(PolygonMode::FILL)
            .line_width(1f32)
            .cull_mode(CullModeFlags::BACK)
            .front_face(FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0f32)
            .depth_bias_clamp(0f32)
            .depth_bias_slope_factor(0f32);
        let multisample_state = PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(SampleCountFlags::TYPE_1)
            .min_sample_shading(1f32)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false);
        let color_blend_attachments = [PipelineColorBlendAttachmentState::builder()
            .color_write_mask(
                ColorComponentFlags::R
                    | ColorComponentFlags::G
                    | ColorComponentFlags::B
                    | ColorComponentFlags::A,
            )
            .blend_enable(false)
            .src_color_blend_factor(BlendFactor::ONE)
            .dst_color_blend_factor(BlendFactor::ZERO)
            .color_blend_op(BlendOp::ADD)
            .src_alpha_blend_factor(BlendFactor::ONE)
            .dst_alpha_blend_factor(BlendFactor::ZERO)
            .alpha_blend_op(BlendOp::ADD)
            .build()];
        let color_blend_state = PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0f32, 0f32, 0f32, 0f32]);
        let main_name = CString::new("main").unwrap();
        let stages = [
            PipelineShaderStageCreateInfo::builder()
                .stage(ShaderStageFlags::VERTEX)
                .module(*context.vertex_shader_module)
                .name(&main_name)
                .build(),
            PipelineShaderStageCreateInfo::builder()
                .stage(ShaderStageFlags::FRAGMENT)
                .module(*context.fragment_shader_module)
                .name(&main_name)
                .build(),
        ];
        let depth_stencil_state = PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(CompareOp::GREATER_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
        let create_infos = [GraphicsPipelineCreateInfo::builder()
            .stages(&stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .depth_stencil_state(&depth_stencil_state)
            .layout(*context.pipeline_layout)
            .render_pass(*render_pass)
            .subpass(0)
            .build()];
        let mut pipelines = context.pipeline_cache.create_graphics(&create_infos)?;
        let pipeline = pipelines.remove(0);
        let images = swapchain.images()?;
        Ok(Self {
            render_pass,
            pipeline,
            images,
            swapchain,
        })
    }
}
