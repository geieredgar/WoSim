use std::sync::Arc;

use util::shader::align_bytes;
use vulkan::{
    BufferUsageFlags, DescriptorSetLayout, DescriptorSetLayoutBinding,
    DescriptorSetLayoutCreateFlags, DescriptorType, Device, GpuVec, MemoryUsage, PipelineLayout,
    PipelineLayoutCreateFlags, ShaderModule, ShaderModuleCreateFlags, ShaderStageFlags,
};

use crate::{
    error::Error,
    shaders::{SCENE_FRAG, SCENE_VERT},
};

use super::{Camera, Mesh, MeshData, Model, Object, Vertex};

pub struct SceneContext {
    pub vertices: GpuVec<Vertex>,
    pub vertex_indices: GpuVec<u32>,
    pub models: GpuVec<Model>,
    pub objects: Vec<Object>,
    pub camera: Camera,
    pub pipeline_layout: PipelineLayout,
    pub set_layout: DescriptorSetLayout,
    pub vertex_shader_module: ShaderModule,
    pub fragment_shader_module: ShaderModule,
}

impl SceneContext {
    pub fn new(
        device: &Arc<Device>,
        vertex_capacity: usize,
        index_capacity: usize,
        model_capacity: usize,
        camera: Camera,
    ) -> Result<Self, Error> {
        let vertex_shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(SCENE_VERT.load()?.bytes()),
        )?;
        let fragment_shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(SCENE_FRAG.load()?.bytes()),
        )?;
        let set_layout = device.create_descriptor_set_layout(
            DescriptorSetLayoutCreateFlags::empty(),
            &[
                DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .descriptor_type(DescriptorType::STORAGE_BUFFER)
                    .stage_flags(ShaderStageFlags::VERTEX)
                    .build(),
                DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .descriptor_count(1)
                    .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                    .stage_flags(ShaderStageFlags::VERTEX)
                    .build(),
            ],
        )?;
        let pipeline_layout = device.create_pipeline_layout(
            PipelineLayoutCreateFlags::empty(),
            &[&set_layout],
            &[],
        )?;
        Ok(Self {
            vertex_shader_module,
            fragment_shader_module,
            set_layout,
            pipeline_layout,
            vertices: device.create_vec(
                vertex_capacity,
                BufferUsageFlags::VERTEX_BUFFER,
                MemoryUsage::CpuToGpu,
            )?,
            vertex_indices: device.create_vec(
                index_capacity,
                BufferUsageFlags::INDEX_BUFFER,
                MemoryUsage::CpuToGpu,
            )?,
            models: device.create_vec(
                model_capacity,
                BufferUsageFlags::STORAGE_BUFFER,
                MemoryUsage::CpuToGpu,
            )?,
            objects: Vec::new(),
            camera,
        })
    }

    pub fn insert_object(&mut self, object: Object) -> u32 {
        let object_index = self.objects.len() as u32;
        self.objects.push(object);
        object_index
    }

    pub fn clear(&mut self) {
        self.objects.clear();
    }

    pub fn insert_mesh(&mut self, mesh: MeshData) -> Mesh {
        let vertex_offset = self.vertices.len() as i32;
        let first_index = self.vertex_indices.len() as u32;
        let index_count = mesh.indices.len() as u32;
        self.vertices.append(&mesh.vertices);
        self.vertex_indices.append(&mesh.indices);
        Mesh {
            _first_index: first_index,
            _index_count: index_count,
            _vertex_offset: vertex_offset,
        }
    }

    pub fn insert_model(&mut self, model: Model) -> u32 {
        let model_index = self.models.len() as u32;
        self.models.push(model);
        model_index
    }

    pub fn flush(&self) -> Result<(), vulkan::Error> {
        self.vertices.flush()?;
        self.vertex_indices.flush()?;
        self.models.flush()?;
        Ok(())
    }
}
