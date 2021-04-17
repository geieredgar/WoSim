use std::sync::Arc;

use egui::{ClippedMesh, CtxRef, Output, RawInput};

use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use wosim_common::{
    shader::align_bytes,
    vulkan::{
        DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags,
        DescriptorType, Device, Filter, PipelineLayout, PipelineLayoutCreateFlags,
        PushConstantRange, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode,
        ShaderModule, ShaderModuleCreateFlags, ShaderStageFlags, LOD_CLAMP_NONE,
    },
};

use crate::{
    error::Error,
    shaders::{EGUI_FRAG, EGUI_VERT},
};

use super::Font;

pub struct EguiContext {
    pub(super) inner: CtxRef,
    input: RawInput,
    pub(super) pipeline_layout: PipelineLayout,
    pub(super) vertex_shader_module: ShaderModule,
    pub(super) fragment_shader_module: ShaderModule,
    pub(super) sampler: Sampler,
    _output: Output,
    pub(super) meshes: Arc<Vec<ClippedMesh>>,
    pub(super) enabled: bool,
    pub(super) set_layout: DescriptorSetLayout,
    pub(super) font: Option<Arc<Font>>,
}

impl EguiContext {
    pub fn new(device: &Arc<Device>) -> Result<Self, Error> {
        let inner = CtxRef::default();
        let input = RawInput::default();
        let vertex_shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(EGUI_VERT.load()?.bytes()),
        )?;
        let fragment_shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(EGUI_FRAG.load()?.bytes()),
        )?;
        let bindings = [DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(ShaderStageFlags::FRAGMENT)
            .build()];
        let set_layout = device
            .create_descriptor_set_layout(DescriptorSetLayoutCreateFlags::empty(), &bindings)?;
        let set_layouts = [&set_layout];
        let push_constant_ranges = [
            PushConstantRange::builder()
                .offset(0)
                .size(8)
                .stage_flags(ShaderStageFlags::VERTEX)
                .build(),
            PushConstantRange::builder()
                .offset(8)
                .size(4)
                .stage_flags(ShaderStageFlags::FRAGMENT)
                .build(),
        ];
        let pipeline_layout = device.create_pipeline_layout(
            PipelineLayoutCreateFlags::empty(),
            &set_layouts,
            &push_constant_ranges,
        )?;
        let output = Output::default();
        let sampler = device.create_sampler(
            &SamplerCreateInfo::builder()
                .address_mode_u(SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(SamplerAddressMode::CLAMP_TO_EDGE)
                .anisotropy_enable(false)
                .min_filter(Filter::LINEAR)
                .mag_filter(Filter::LINEAR)
                .mipmap_mode(SamplerMipmapMode::LINEAR)
                .min_lod(0.0)
                .max_lod(LOD_CLAMP_NONE),
        )?;
        Ok(Self {
            inner,
            input,
            _output: output,
            meshes: Arc::new(Vec::new()),
            vertex_shader_module,
            fragment_shader_module,
            pipeline_layout,
            enabled: false,
            set_layout,
            sampler,
            font: None,
        })
    }

    pub fn handle_event(&mut self, event: &Event<()>) {
        if !self.enabled {
            if let Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } = event
            {
                if let Some(VirtualKeyCode::F) = input.virtual_keycode {
                    if input.state == ElementState::Pressed {
                        self.enabled = true;
                    }
                }
            }
            return;
        }
        if let Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } = event
        {
            if let Some(VirtualKeyCode::F1) = input.virtual_keycode {
                if input.state == ElementState::Pressed {
                    self.enabled = false;
                }
            }
        }
    }

    pub fn frame<F: Fn(&CtxRef)>(&mut self, builder: F) {
        if !self.enabled {
            return;
        }
        self.inner.begin_frame(self.input.take());
        builder(&self.inner);
        let (output, shapes) = self.inner.end_frame();
        let meshes = Arc::new(self.inner.tessellate(shapes));
        self._output = output;
        self.meshes = meshes;
    }
}
