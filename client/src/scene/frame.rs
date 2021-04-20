use std::{mem::size_of, sync::Arc};

use nalgebra::base::Matrix4;
use wosim_common_vulkan::{
    AllocationCreateFlags, AllocationCreateInfo, Buffer, BufferCreateInfo, BufferUsageFlags,
    CommandBuffer, DescriptorBufferInfo, DescriptorPool, DescriptorPoolSetup, DescriptorSet,
    DescriptorType, Device, DrawIndexedIndirectCommand, Extent2D, GpuVariable, GpuVec, IndexType,
    MemoryPropertyFlags, MemoryUsage, PipelineBindPoint, SubpassContents, WriteDescriptorSet,
    WHOLE_SIZE,
};

use super::{DrawData, Object, SceneConstants, SceneContext, SceneView};

pub struct SceneFrame {
    pub descriptor_set: DescriptorSet,
    pub objects: GpuVec<Object>,
    pub constants: GpuVariable<SceneConstants>,
    pub draw_count_read_back: GpuVariable<u32>,
    pub draw_count: Buffer,
    pub draw_data: Buffer,
    pub commands: Buffer,
    pub previous_view: Matrix4<f32>,
}

impl SceneFrame {
    pub fn new(
        device: &Arc<Device>,
        context: &SceneContext,
        object_capacity: usize,
        descriptor_pool: &DescriptorPool,
    ) -> Result<Self, wosim_common_vulkan::Error> {
        let mut descriptor_sets = descriptor_pool.allocate(&[&context.set_layout])?;
        let descriptor_set = descriptor_sets.remove(0);
        let constants = device.create_variable(
            BufferUsageFlags::UNIFORM_BUFFER,
            MemoryUsage::CpuToGpu,
            SceneConstants::default(),
        )?;
        let allocation_create_info = AllocationCreateInfo {
            usage: MemoryUsage::GpuOnly,
            flags: AllocationCreateFlags::empty(),
            required_flags: MemoryPropertyFlags::empty(),
            preferred_flags: MemoryPropertyFlags::empty(),
            memory_type_bits: 0,
            pool: None,
            user_data: None,
        };
        let create_info = BufferCreateInfo::builder()
            .size((size_of::<DrawIndexedIndirectCommand>() * object_capacity) as u64)
            .usage(BufferUsageFlags::STORAGE_BUFFER | BufferUsageFlags::INDIRECT_BUFFER);
        let (commands, _) = device.create_buffer(&create_info, &allocation_create_info)?;
        let create_info = BufferCreateInfo::builder()
            .size((size_of::<DrawData>() * object_capacity) as u64)
            .usage(BufferUsageFlags::STORAGE_BUFFER);
        let (draw_data, _) = device.create_buffer(&create_info, &allocation_create_info)?;
        let create_info = BufferCreateInfo::builder()
            .size(size_of::<u32>() as u64)
            .usage(
                BufferUsageFlags::STORAGE_BUFFER
                    | BufferUsageFlags::TRANSFER_SRC
                    | BufferUsageFlags::TRANSFER_DST
                    | BufferUsageFlags::INDIRECT_BUFFER,
            );
        let (draw_count, _) = device.create_buffer(&create_info, &allocation_create_info)?;
        let constants_buffer_info = [DescriptorBufferInfo::builder()
            .offset(0)
            .range(WHOLE_SIZE)
            .buffer(**constants.buffer())
            .build()];
        let draw_data_buffer_info = [DescriptorBufferInfo::builder()
            .offset(0)
            .range(WHOLE_SIZE)
            .buffer(*draw_data)
            .build()];
        let descriptor_writes = [
            WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(DescriptorType::STORAGE_BUFFER)
                .buffer_info(&draw_data_buffer_info)
                .build(),
            WriteDescriptorSet::builder()
                .dst_set(*descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&constants_buffer_info)
                .build(),
        ];
        device.update_descriptor_sets(&descriptor_writes, &[]);
        Ok(Self {
            descriptor_set,
            objects: device.create_vec(
                object_capacity,
                BufferUsageFlags::STORAGE_BUFFER,
                MemoryUsage::CpuToGpu,
            )?,
            draw_count_read_back: device.create_variable(
                BufferUsageFlags::TRANSFER_DST,
                MemoryUsage::GpuToCpu,
                0,
            )?,
            commands,
            constants,
            draw_count,
            draw_data,
            previous_view: Matrix4::identity(),
        })
    }

    pub fn update(
        &mut self,
        context: &SceneContext,
        extent: Extent2D,
    ) -> Result<u32, wosim_common_vulkan::Error> {
        let aspect = (extent.width as f32) / (extent.height as f32);
        let h = (context.camera.fovy / 2.0).tan();
        let w = h * aspect;
        let projection = Matrix4::new(
            1.0 / w,
            0.0,
            0.0,
            0.0,
            0.0,
            -1.0 / h,
            0.0,
            0.0,
            0.0,
            0.0,
            context.camera.znear / (context.camera.zfar - context.camera.znear),
            context.camera.znear * context.camera.zfar
                / (context.camera.zfar - context.camera.znear),
            0.0,
            0.0,
            -1.0,
            0.0,
        );
        let view = context.camera.rotation().inverse().to_homogeneous()
            * context.camera.translation.inverse().to_homogeneous();
        let previous_view = self.previous_view;
        self.previous_view = view;
        let view_projection = projection * view;
        *self.constants.value_mut() = SceneConstants {
            object_count: context.objects.len() as u32,
            view,
            previous_view,
            projection,
            view_projection,
            zfar: context.camera.zfar,
            znear: context.camera.znear,
            w,
            h,
        };
        self.constants.flush()?;
        self.objects.clear();
        self.objects.append(&context.objects);
        self.objects.flush()?;
        self.draw_count_read_back.invalidate()?;
        Ok(*self.draw_count_read_back.value())
    }

    pub fn render(
        &self,
        command_buffer: &CommandBuffer,
        context: &SceneContext,
        view: &SceneView,
        use_draw_count: bool,
    ) {
        command_buffer.bind_descriptor_sets(
            PipelineBindPoint::GRAPHICS,
            &context.pipeline_layout,
            0,
            &[&self.descriptor_set],
            &[],
        );
        command_buffer.bind_vertex_buffers(0, &[(context.vertices.buffer(), 0)]);
        command_buffer.bind_index_buffer(context.vertex_indices.buffer(), 0, IndexType::UINT32);
        command_buffer.bind_pipeline(PipelineBindPoint::GRAPHICS, &view.pre_pass_pipeline);
        self.draw(command_buffer, use_draw_count);
        command_buffer.next_subpass(SubpassContents::INLINE);
        command_buffer.bind_pipeline(PipelineBindPoint::GRAPHICS, &view.pipeline);
        self.draw(command_buffer, use_draw_count);
    }

    fn draw(&self, command_buffer: &CommandBuffer, use_draw_count: bool) {
        if use_draw_count {
            command_buffer.draw_indexed_indirect_count(
                &self.commands,
                0,
                &self.draw_count,
                0,
                self.objects.len() as u32,
                size_of::<DrawIndexedIndirectCommand>() as u32,
            )
        } else {
            command_buffer.draw_indexed_indirect(
                &self.commands,
                0,
                self.objects.len() as u32,
                size_of::<DrawIndexedIndirectCommand>() as u32,
            )
        }
    }

    pub fn pool_setup() -> DescriptorPoolSetup {
        DescriptorPoolSetup {
            combined_image_samplers: 0,
            sets: 1,
            storage_buffers: 1,
            storage_images: 0,
            uniform_buffers: 1,
        }
    }
}
