use std::{ffi::CString, mem::size_of, sync::Arc};

use bytemuck::{bytes_of, Pod, Zeroable};
use wosim_common_base::shader::align_bytes;
use wosim_common_vulkan::{
    Bool32, ComputePipelineCreateInfo, DescriptorSetLayout, DescriptorSetLayoutBinding,
    DescriptorSetLayoutCreateFlags, DescriptorType, Device, Pipeline, PipelineCache,
    PipelineLayout, PipelineLayoutCreateFlags, PipelineShaderStageCreateInfo, ShaderModule,
    ShaderModuleCreateFlags, ShaderStageFlags, SpecializationInfo, SpecializationMapEntry,
};

use crate::{error::Error, shaders::CULL_COMP};

pub struct CullContext {
    pub pipeline: Pipeline,
    pub pipeline_layout: PipelineLayout,
    pub set_layout: DescriptorSetLayout,
    pub shader_module: ShaderModule,
}
#[derive(Clone, Copy)]
struct CullSpecializationConstants {
    _use_draw_count: Bool32,
}

unsafe impl Zeroable for CullSpecializationConstants {}
unsafe impl Pod for CullSpecializationConstants {}

impl CullContext {
    pub fn new(
        device: &Arc<Device>,
        use_draw_count: Bool32,
        pipeline_cache: &PipelineCache,
    ) -> Result<Self, Error> {
        let shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(CULL_COMP.load()?.bytes()),
        )?;
        let bindings = [
            DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
            DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
            DescriptorSetLayoutBinding::builder()
                .binding(2)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
            DescriptorSetLayoutBinding::builder()
                .binding(3)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
            DescriptorSetLayoutBinding::builder()
                .binding(4)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
            DescriptorSetLayoutBinding::builder()
                .binding(5)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .stage_flags(ShaderStageFlags::COMPUTE)
                .build(),
            DescriptorSetLayoutBinding::builder()
                .binding(6)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
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
        let specialization_constants = CullSpecializationConstants {
            _use_draw_count: use_draw_count,
        };
        let map_entries = [SpecializationMapEntry::builder()
            .constant_id(0)
            .offset(0)
            .size(size_of::<Bool32>())
            .build()];
        let specialization_info = SpecializationInfo::builder()
            .data(bytes_of(&specialization_constants))
            .map_entries(&map_entries);
        let stage = PipelineShaderStageCreateInfo::builder()
            .stage(ShaderStageFlags::COMPUTE)
            .module(*shader_module)
            .name(&main_name)
            .specialization_info(&specialization_info)
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
