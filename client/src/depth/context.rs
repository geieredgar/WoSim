use std::{ffi::CString, sync::Arc};

use common::shader::align_bytes;
use vulkan::{
    ComputePipelineCreateInfo, DescriptorSetLayout, DescriptorSetLayoutBinding,
    DescriptorSetLayoutCreateFlags, DescriptorType, Device, Pipeline, PipelineCache,
    PipelineLayout, PipelineLayoutCreateFlags, PipelineShaderStageCreateInfo, ShaderModule,
    ShaderModuleCreateFlags, ShaderStageFlags,
};

use crate::{error::Error, shaders::DEPTH_PYRAMID_COMP};

pub struct DepthContext {
    pub pipeline: Pipeline,
    pub pipeline_layout: PipelineLayout,
    pub set_layout: DescriptorSetLayout,
    pub shader_module: ShaderModule,
}

impl DepthContext {
    pub fn new(device: &Arc<Device>, pipeline_cache: &PipelineCache) -> Result<Self, Error> {
        let shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(DEPTH_PYRAMID_COMP.load()?.bytes()),
        )?;
        let bindings = [
            DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
            DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::STORAGE_IMAGE)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
        ];
        let set_layout = device
            .create_descriptor_set_layout(DescriptorSetLayoutCreateFlags::empty(), &bindings)?;
        let pipeline_layout = device.create_pipeline_layout(
            PipelineLayoutCreateFlags::empty(),
            &[&set_layout],
            &[],
        )?;
        let main_name = CString::new("main").unwrap();
        let stage = PipelineShaderStageCreateInfo::builder()
            .stage(ShaderStageFlags::COMPUTE)
            .module(*shader_module)
            .name(&main_name)
            .build();
        let create_infos = [ComputePipelineCreateInfo::builder()
            .stage(stage)
            .layout(*pipeline_layout)
            .build()];
        let mut pipelines = pipeline_cache.create_compute(&create_infos)?;
        let pipeline = pipelines.remove(0);
        Ok(Self {
            pipeline,
            pipeline_layout,
            set_layout,
            shader_module,
        })
    }
}
