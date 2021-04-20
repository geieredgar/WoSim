use std::{ffi::CString, mem::size_of};

use wosim_common::vulkan::{
    BlendFactor, BlendOp, ColorComponentFlags, CompareOp, CullModeFlags, Extent2D, Format,
    FrontFace, GraphicsPipelineCreateInfo, LogicOp, Offset2D, Pipeline, PipelineCache,
    PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo,
    PipelineDepthStencilStateCreateInfo, PipelineInputAssemblyStateCreateInfo,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateInfo,
    PipelineShaderStageCreateInfo, PipelineVertexInputStateCreateInfo,
    PipelineViewportStateCreateInfo, PolygonMode, PrimitiveTopology, Rect2D, RenderPass,
    SampleCountFlags, ShaderStageFlags, VertexInputAttributeDescription,
    VertexInputBindingDescription, VertexInputRate, Viewport, VkResult,
};

use super::{SceneContext, Vertex};

pub struct SceneView {
    pub pre_pass_pipeline: Pipeline,
    pub pipeline: Pipeline,
}

impl SceneView {
    pub fn new(
        context: &SceneContext,
        render_pass: &RenderPass,
        pipeline_cache: &PipelineCache,
        first_subpass: u32,
        image_extent: Extent2D,
    ) -> VkResult<Self> {
        let binding_descriptions = [VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Vertex>() as u32)
            .input_rate(VertexInputRate::VERTEX)
            .build()];
        let attribute_descriptions = [
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(Format::R32G32B32_SFLOAT)
                .offset(0)
                .build(),
            VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(Format::R32G32B32_SFLOAT)
                .offset(12)
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
        let pre_pass_stages = [PipelineShaderStageCreateInfo::builder()
            .stage(ShaderStageFlags::VERTEX)
            .module(*context.vertex_shader_module)
            .name(&main_name)
            .build()];
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
        let pre_pass_depth_stencil_state = PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(CompareOp::GREATER)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
        let depth_stencil_state = PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(false)
            .depth_compare_op(CompareOp::GREATER_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false);
        let create_infos = [
            GraphicsPipelineCreateInfo::builder()
                .stages(&pre_pass_stages)
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_state)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample_state)
                .color_blend_state(&color_blend_state)
                .depth_stencil_state(&pre_pass_depth_stencil_state)
                .layout(*context.pipeline_layout)
                .render_pass(**render_pass)
                .subpass(first_subpass)
                .build(),
            GraphicsPipelineCreateInfo::builder()
                .stages(&stages)
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_state)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization_state)
                .multisample_state(&multisample_state)
                .color_blend_state(&color_blend_state)
                .depth_stencil_state(&depth_stencil_state)
                .layout(*context.pipeline_layout)
                .render_pass(**render_pass)
                .subpass(first_subpass + 1)
                .build(),
        ];
        let mut pipelines = pipeline_cache.create_graphics(&create_infos)?;
        let pre_pass_pipeline = pipelines.remove(0);
        let pipeline = pipelines.remove(0);
        Ok(Self {
            pre_pass_pipeline,
            pipeline,
        })
    }
}
