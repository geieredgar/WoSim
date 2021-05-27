use std::sync::Arc;

use vulkan::{
    ApiResult, DescriptorBufferInfo, DescriptorImageInfo, DescriptorPool, DescriptorPoolSetup,
    DescriptorSet, DescriptorType, Device, ImageLayout, WriteDescriptorSet, WHOLE_SIZE,
};

use crate::{
    depth::DepthView,
    scene::{SceneContext, SceneFrame},
};

use super::CullContext;

pub struct CullFrame {
    pub descriptor_set: DescriptorSet,
}

impl CullFrame {
    pub fn new(
        device: &Arc<Device>,
        context: &CullContext,
        scene_context: &SceneContext,
        scene_frame: &SceneFrame,
        descriptor_pool: &DescriptorPool,
    ) -> Result<Self, ApiResult> {
        let mut descriptor_sets = descriptor_pool.allocate(&[&context.set_layout])?;
        let descriptor_set = descriptor_sets.remove(0);
        let constants_buffer_info = [DescriptorBufferInfo::builder()
            .offset(0)
            .range(WHOLE_SIZE)
            .buffer(**scene_frame.constants.buffer())
            .build()];
        let model_buffer_info = [DescriptorBufferInfo::builder()
            .offset(0)
            .range(WHOLE_SIZE)
            .buffer(**scene_context.models.buffer())
            .build()];
        let objects_buffer_info = [DescriptorBufferInfo::builder()
            .offset(0)
            .range(WHOLE_SIZE)
            .buffer(**scene_frame.objects.buffer())
            .build()];
        let draw_count_buffer_info = [DescriptorBufferInfo::builder()
            .offset(0)
            .range(WHOLE_SIZE)
            .buffer(*scene_frame.draw_count)
            .build()];
        let commands_buffer_info = [DescriptorBufferInfo::builder()
            .offset(0)
            .range(WHOLE_SIZE)
            .buffer(*scene_frame.commands)
            .build()];
        let draw_data_buffer_info = [DescriptorBufferInfo::builder()
            .offset(0)
            .range(WHOLE_SIZE)
            .buffer(*scene_frame.draw_data)
            .build()];
        let descriptor_writes = [
            WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&constants_buffer_info)
                .build(),
            WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .buffer_info(&objects_buffer_info)
                .build(),
            WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(3)
                .dst_array_element(0)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .buffer_info(&model_buffer_info)
                .build(),
            WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(4)
                .dst_array_element(0)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .buffer_info(&draw_count_buffer_info)
                .build(),
            WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(5)
                .dst_array_element(0)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .buffer_info(&commands_buffer_info)
                .build(),
            WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(6)
                .dst_array_element(0)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .buffer_info(&draw_data_buffer_info)
                .build(),
        ];
        device.update_descriptor_sets(&descriptor_writes, &[]);
        Ok(Self { descriptor_set })
    }

    pub fn setup_view(&mut self, device: &Arc<Device>, depth_view: &DepthView) {
        let image_info = [DescriptorImageInfo::builder()
            .sampler(*depth_view.sampler)
            .image_view(*depth_view.pyramid_view)
            .image_layout(ImageLayout::GENERAL)
            .build()];
        let descriptor_writes = [WriteDescriptorSet::builder()
            .dst_set(*self.descriptor_set)
            .dst_binding(1)
            .dst_array_element(0)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_info)
            .build()];
        device.update_descriptor_sets(&descriptor_writes, &[]);
    }

    pub fn pool_setup() -> DescriptorPoolSetup {
        DescriptorPoolSetup {
            storage_buffers: 5,
            uniform_buffers: 1,
            sets: 1,
            combined_image_samplers: 1,
            storage_images: 0,
        }
    }
}
