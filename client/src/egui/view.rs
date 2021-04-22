use std::{ffi::CString, sync::Arc};

use vulkan::{
    BlendFactor, BlendOp, ColorComponentFlags, CullModeFlags, Device, DynamicState, Extent2D,
    Format, GraphicsPipelineCreateInfo, LogicOp, Offset2D, Pipeline, PipelineCache,
    PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineDepthStencilStateCreateInfo, PipelineDynamicStateCreateInfo,
    PipelineInputAssemblyStateCreateInfo, PipelineMultisampleStateCreateInfo,
    PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateInfo,
    PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateInfo, PolygonMode,
    PrimitiveTopology, Rect2D, RenderPass, SampleCountFlags, ShaderStageFlags,
    VertexInputAttributeDescription, VertexInputBindingDescription, VertexInputRate, Viewport,
};

use crate::error::Error;

use super::EguiContext;

pub struct EguiView {
    pub(super) pipeline: Pipeline,
}

impl EguiView {
    pub fn new(
        _device: &Arc<Device>,
        context: &EguiContext,
        pipeline_cache: &PipelineCache,
        image_extent: Extent2D,
        _image_format: Format,
        render_pass: &RenderPass,
        subpass_index: u32,
    ) -> Result<Self, Error> {
        let binding_descriptions = [VertexInputBindingDescription::builder()
            .binding(0)
            .input_rate(VertexInputRate::VERTEX)
            .stride(20)
            .build()];
        let attribute_descriptions = [
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(Format::R32G32_SFLOAT)
                .offset(0)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(Format::R32G32_SFLOAT)
                .offset(8)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(Format::R8G8B8A8_UNORM)
                .offset(16)
                .build(),
        ];
        let vertex_input_state = PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);
        let input_assembly_state = PipelineInputAssemblyStateCreateInfo::builder()
            .topology(PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
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
            .cull_mode(CullModeFlags::NONE)
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
            .blend_enable(true)
            .src_color_blend_factor(BlendFactor::ONE)
            .dst_color_blend_factor(BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(BlendOp::ADD)
            .src_alpha_blend_factor(BlendFactor::ZERO)
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
        let dynamic_states = [DynamicState::SCISSOR];
        let dynamic_state =
            PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);
        let depth_stencil_state = PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
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
            .dynamic_state(&dynamic_state)
            .render_pass(**render_pass)
            .subpass(subpass_index)
            .build()];
        let mut pipelines = pipeline_cache.create_graphics(&create_infos)?;
        let pipeline = pipelines.remove(0);
        Ok(Self { pipeline })
    }
}
