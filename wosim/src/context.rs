use std::sync::Arc;

use wosim_common::{
    shader::align_bytes,
    vulkan::{
        Device, PipelineCache, PipelineCacheCreateFlags, PipelineLayout, PipelineLayoutCreateFlags,
        ShaderModule, ShaderModuleCreateFlags,
    },
};

use crate::{
    egui::EguiContext,
    error::Error,
    renderer::RenderConfiguration,
    shaders::{DEFAULT_FRAG, DEFAULT_VERT},
};

pub struct Context {
    pub vertex_shader_module: ShaderModule,
    pub fragment_shader_module: ShaderModule,
    pub pipeline_cache: PipelineCache,
    pub pipeline_layout: PipelineLayout,
    pub configuration: RenderConfiguration,
    pub egui: EguiContext,
}

impl Context {
    pub fn new(device: &Arc<Device>, configuration: RenderConfiguration) -> Result<Self, Error> {
        let vertex_shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(DEFAULT_VERT.load()?.bytes()),
        )?;
        let fragment_shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(DEFAULT_FRAG.load()?.bytes()),
        )?;
        let pipeline_cache =
            device.create_pipeline_cache(PipelineCacheCreateFlags::empty(), None)?;
        let set_layouts = [];
        let pipeline_layout =
            device.create_pipeline_layout(PipelineLayoutCreateFlags::empty(), &set_layouts, &[])?;
        let egui = EguiContext::new(device)?;
        Ok(Self {
            vertex_shader_module,
            fragment_shader_module,
            pipeline_cache,
            pipeline_layout,
            configuration,
            egui,
        })
    }
}
