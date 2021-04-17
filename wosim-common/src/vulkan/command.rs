use ash::{
    prelude::VkResult,
    version::DeviceV1_0,
    vk::{
        self, AccessFlags, BufferImageCopy, BufferMemoryBarrier, ClearValue,
        CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferInheritanceInfo,
        CommandBufferLevel, CommandBufferUsageFlags, CommandPoolResetFlags, DependencyFlags,
        ImageLayout, ImageMemoryBarrier, ImageSubresourceRange, IndexType, MemoryBarrier,
        PipelineBindPoint, PipelineStageFlags, Rect2D, RenderPassBeginInfo, ShaderStageFlags,
        SubpassContents,
    },
};
use bytemuck::{bytes_of, Pod};

use super::{
    Buffer, DescriptorSet, Framebuffer, Image, Object, Pipeline, PipelineLayout, RenderPass,
};

pub type CommandPool = Object<vk::CommandPool>;
pub type CommandBuffer = Object<vk::CommandBuffer>;

impl CommandPool {
    pub fn allocate(&self, level: CommandBufferLevel, count: u32) -> VkResult<Vec<CommandBuffer>> {
        let create_info = CommandBufferAllocateInfo::builder()
            .command_pool(self.handle)
            .level(level)
            .command_buffer_count(count);
        Ok(
            unsafe { self.device.inner.allocate_command_buffers(&create_info) }?
                .into_iter()
                .map(|handle| CommandBuffer {
                    handle,
                    device: self.device.clone(),
                })
                .collect(),
        )
    }

    pub fn allocate_single_primary(&self) -> VkResult<CommandBuffer> {
        Ok(self.allocate(CommandBufferLevel::PRIMARY, 1)?.remove(0))
    }

    pub fn reset(&self, flags: CommandPoolResetFlags) -> VkResult<()> {
        unsafe { self.device.inner.reset_command_pool(self.handle, flags) }
    }
}

impl CommandBuffer {
    pub fn begin(
        &self,
        flags: CommandBufferUsageFlags,
        inheritance: Option<&CommandBufferInheritanceInfo>,
    ) -> VkResult<()> {
        let begin_info = if let Some(inheritance) = inheritance {
            CommandBufferBeginInfo::builder().inheritance_info(inheritance)
        } else {
            CommandBufferBeginInfo::builder()
        }
        .flags(flags);
        unsafe {
            self.device
                .inner
                .begin_command_buffer(self.handle, &begin_info)
        }
    }

    pub fn begin_render_pass(
        &self,
        render_pass: &RenderPass,
        framebuffer: &Framebuffer,
        render_area: Rect2D,
        clear_values: &[ClearValue],
        contents: SubpassContents,
    ) {
        let create_info = RenderPassBeginInfo::builder()
            .render_pass(render_pass.handle)
            .framebuffer(framebuffer.handle)
            .render_area(render_area)
            .clear_values(clear_values);
        unsafe {
            self.device
                .inner
                .cmd_begin_render_pass(self.handle, &create_info, contents)
        }
    }

    pub fn copy_buffer_to_image(
        &self,
        src_buffer: &Buffer,
        dst_image: &Image,
        dst_image_layout: ImageLayout,
        regions: &[BufferImageCopy],
    ) {
        unsafe {
            self.device.inner.cmd_copy_buffer_to_image(
                self.handle,
                src_buffer.handle,
                dst_image.handle,
                dst_image_layout,
                regions,
            )
        }
    }

    pub fn next_subpass(&self, contents: SubpassContents) {
        unsafe { self.device.inner.cmd_next_subpass(self.handle, contents) }
    }

    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.inner.cmd_draw(
                self.handle,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            )
        };
    }

    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.inner.cmd_draw_indexed(
                self.handle,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            )
        }
    }

    pub fn bind_index_buffer(&self, buffer: &Buffer, offset: u64, index_type: IndexType) {
        unsafe {
            self.device
                .inner
                .cmd_bind_index_buffer(self.handle, buffer.handle, offset, index_type)
        }
    }

    pub fn bind_vertex_buffers(&self, first_binding: u32, buffers: &[(&Buffer, u64)]) {
        let offsets: Vec<_> = buffers.iter().map(|(_, offset)| *offset).collect();
        let buffers: Vec<_> = buffers.iter().map(|(buffer, _)| buffer.handle).collect();
        unsafe {
            self.device.inner.cmd_bind_vertex_buffers(
                self.handle,
                first_binding,
                &buffers,
                &offsets,
            )
        }
    }

    pub fn push_constants<T: Pod>(
        &self,
        layout: &PipelineLayout,
        stage_flags: ShaderStageFlags,
        offset: u32,
        constant: &T,
    ) {
        unsafe {
            self.device.inner.cmd_push_constants(
                self.handle,
                layout.handle,
                stage_flags,
                offset,
                bytes_of(constant),
            )
        }
    }

    pub fn bind_descriptor_sets(
        &self,
        pipeline_bind_point: PipelineBindPoint,
        layout: &PipelineLayout,
        first_set: u32,
        descriptor_sets: &[&DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        let descriptor_sets: Vec<_> = descriptor_sets
            .iter()
            .map(|descriptor_set| descriptor_set.handle)
            .collect();
        unsafe {
            self.device.inner.cmd_bind_descriptor_sets(
                self.handle,
                pipeline_bind_point,
                layout.handle,
                first_set,
                &descriptor_sets,
                dynamic_offsets,
            )
        }
    }

    pub fn end_render_pass(&self) {
        unsafe { self.device.inner.cmd_end_render_pass(self.handle) }
    }

    pub fn bind_pipeline(&self, binding_point: PipelineBindPoint, pipeline: &Pipeline) {
        unsafe {
            self.device
                .inner
                .cmd_bind_pipeline(self.handle, binding_point, pipeline.handle)
        }
    }

    pub fn end(&self) -> VkResult<()> {
        unsafe { self.device.inner.end_command_buffer(self.handle) }
    }

    pub fn set_scissor(&self, first_scissor: u32, scissors: &[Rect2D]) {
        unsafe {
            self.device
                .inner
                .cmd_set_scissor(self.handle, first_scissor, scissors)
        }
    }

    pub fn pipeline_barrier(
        &self,
        src_stage_mask: PipelineStageFlags,
        dst_stage_mask: PipelineStageFlags,
        dependency_flags: DependencyFlags,
        memory_barriers: &[MemoryBarrier],
        buffer_memory_barriers: &[BufferMemoryBarrier],
        image_memory_barriers: &[ImageMemoryBarrier],
    ) {
        unsafe {
            self.device.inner.cmd_pipeline_barrier(
                self.handle,
                src_stage_mask,
                dst_stage_mask,
                dependency_flags,
                memory_barriers,
                buffer_memory_barriers,
                image_memory_barriers,
            )
        }
    }

    pub fn transfer_buffer_to_image(
        &self,
        src: &Buffer,
        dst: &Image,
        info: ImageTransferInfo,
        subresource_range: ImageSubresourceRange,
        regions: &[BufferImageCopy],
    ) {
        self.pipeline_barrier(
            PipelineStageFlags::HOST,
            PipelineStageFlags::TRANSFER,
            DependencyFlags::empty(),
            &[],
            &[],
            &[ImageMemoryBarrier::builder()
                .image(dst.handle)
                .src_access_mask(AccessFlags::empty())
                .dst_access_mask(AccessFlags::TRANSFER_WRITE)
                .old_layout(info.initial_layout)
                .new_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
                .subresource_range(subresource_range)
                .build()],
        );
        self.copy_buffer_to_image(src, dst, ImageLayout::TRANSFER_DST_OPTIMAL, regions);
        self.pipeline_barrier(
            PipelineStageFlags::TRANSFER,
            info.dst_stage_mask,
            DependencyFlags::empty(),
            &[],
            &[],
            &[ImageMemoryBarrier::builder()
                .src_access_mask(AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(info.dst_access_mask)
                .old_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(info.final_layout)
                .image(dst.handle)
                .subresource_range(subresource_range)
                .build()],
        );
    }
}

pub struct ImageTransferInfo {
    pub dst_stage_mask: PipelineStageFlags,
    pub dst_access_mask: AccessFlags,
    pub initial_layout: ImageLayout,
    pub final_layout: ImageLayout,
}
