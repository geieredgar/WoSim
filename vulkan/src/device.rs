use std::{ffi::CStr, sync::Arc};

use ash::{
    prelude::VkResult,
    version::DeviceV1_0,
    vk::{
        self, BufferCreateInfo, BufferUsageFlags, CommandPoolCreateFlags, CommandPoolCreateInfo,
        ComponentMapping, CopyDescriptorSet, DescriptorPoolCreateFlags, DescriptorPoolCreateInfo,
        DescriptorPoolSize, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags,
        DescriptorSetLayoutCreateInfo, FenceCreateFlags, FenceCreateInfo, Format, ImageCreateInfo,
        ImageSubresourceRange, ImageViewCreateFlags, ImageViewCreateInfo, ImageViewType,
        PipelineCacheCreateFlags, PipelineCacheCreateInfo, PipelineLayoutCreateFlags,
        PipelineLayoutCreateInfo, PushConstantRange, QueryPipelineStatisticFlags,
        QueryPoolCreateInfo, QueryType, Queue, RenderPassCreateInfo, SamplerCreateInfo,
        SemaphoreCreateInfo, ShaderModuleCreateFlags, ShaderModuleCreateInfo, SubmitInfo,
        WriteDescriptorSet,
    },
};

use vk_mem::{
    AllocationCreateInfo, AllocationInfo, Allocator, AllocatorCreateFlags, AllocatorCreateInfo,
    MemoryUsage,
};

use super::{
    Buffer, CommandPool, DescriptorPool, DescriptorSetLayout, Error, Fence, GpuVariable, GpuVec,
    Handle, Image, ImageView, PhysicalDevice, PhysicalDeviceFeatures, PipelineCache,
    PipelineLayout, QueryPool, RenderPass, Sampler, Semaphore, ShaderModule, Swapchain,
    SwapchainConfiguration,
};

pub struct Device {
    transfer_queue: Option<DeviceQueue>,
    pub(super) main_queue: DeviceQueue,
    pub(super) allocator: Allocator,
    pub(super) inner: ash::Device,
    physical_device: PhysicalDevice,
}

impl Device {
    pub(super) fn new(
        physical_device: PhysicalDevice,
        inner: ash::Device,
        configuration: DeviceConfiguration,
    ) -> vk_mem::Result<Device> {
        let main_queue = DeviceQueue {
            handle: unsafe { inner.get_device_queue(configuration.main_queue_family_index, 0) },
            family_index: configuration.main_queue_family_index,
        };
        let transfer_queue = configuration
            .transfer_queue_family_index
            .map(|family_index| DeviceQueue {
                handle: unsafe { inner.get_device_queue(family_index, 0) },
                family_index,
            });
        let allocator = vk_mem::Allocator::new(&AllocatorCreateInfo {
            physical_device: physical_device.handle,
            device: inner.clone(),
            instance: physical_device.instance.inner.clone(),
            flags: AllocatorCreateFlags::empty(),
            preferred_large_heap_block_size: 0,
            frame_in_use_count: 0,
            heap_size_limits: None,
        })?;
        Ok(Self {
            transfer_queue,
            main_queue,
            allocator,
            inner,
            physical_device,
        })
    }

    pub fn physical_device(&self) -> &PhysicalDevice {
        &self.physical_device
    }

    pub(super) fn destroy_handle<T: Handle>(&self, handle: T) {
        unsafe { handle.destroy(&self.inner) }
    }

    pub fn create_swapchain(
        self: &Arc<Self>,
        configuration: SwapchainConfiguration<'_>,
    ) -> VkResult<Swapchain> {
        Swapchain::new(self.clone(), configuration)
    }

    pub fn submit(&self, submits: &[SubmitInfo], fence: &Fence) -> VkResult<()> {
        unsafe {
            self.inner
                .queue_submit(self.main_queue.handle, submits, fence.handle)
        }
    }

    pub fn submit_without_fence(&self, submits: &[SubmitInfo]) -> VkResult<()> {
        unsafe {
            self.inner
                .queue_submit(self.main_queue.handle, submits, vk::Fence::null())
        }
    }

    pub fn transfer_submit(&self, submits: &[SubmitInfo], fence: Option<&Fence>) -> VkResult<()> {
        unsafe {
            self.inner.queue_submit(
                self.transfer_queue
                    .as_ref()
                    .unwrap_or(&self.main_queue)
                    .handle,
                submits,
                fence.map(|x| x.handle).unwrap_or_else(vk::Fence::null),
            )
        }
    }

    pub fn main_queue_family_index(&self) -> u32 {
        self.main_queue.family_index
    }

    pub fn transfer_queue_family_index(&self) -> u32 {
        self.transfer_queue
            .as_ref()
            .unwrap_or(&self.main_queue)
            .family_index
    }

    pub fn has_dedicated_transfer_queue(&self) -> bool {
        self.transfer_queue.is_some()
    }

    pub fn create_command_pool(
        self: &Arc<Self>,
        flags: CommandPoolCreateFlags,
        queue_family_index: u32,
    ) -> VkResult<CommandPool> {
        let create_info = CommandPoolCreateInfo::builder()
            .flags(flags)
            .queue_family_index(queue_family_index);
        let handle = unsafe { self.inner.create_command_pool(&create_info, None) }?;
        Ok(CommandPool {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_fence(self: &Arc<Self>, flags: FenceCreateFlags) -> VkResult<Fence> {
        let create_info = FenceCreateInfo::builder().flags(flags);
        let handle = unsafe { self.inner.create_fence(&create_info, None) }?;
        Ok(Fence {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_semaphore(self: &Arc<Self>) -> VkResult<Semaphore> {
        let create_info = SemaphoreCreateInfo::builder();
        let handle = unsafe { self.inner.create_semaphore(&create_info, None) }?;
        Ok(Semaphore {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_descriptor_set_layout(
        self: &Arc<Self>,
        flags: DescriptorSetLayoutCreateFlags,
        bindings: &[DescriptorSetLayoutBinding],
    ) -> VkResult<DescriptorSetLayout> {
        let create_info = DescriptorSetLayoutCreateInfo::builder()
            .flags(flags)
            .bindings(bindings);
        let handle = unsafe { self.inner.create_descriptor_set_layout(&create_info, None) }?;
        Ok(DescriptorSetLayout {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_descriptor_pool(
        self: &Arc<Self>,
        flags: DescriptorPoolCreateFlags,
        pool_sizes: &[DescriptorPoolSize],
        max_sets: u32,
    ) -> VkResult<DescriptorPool> {
        let create_info = DescriptorPoolCreateInfo::builder()
            .flags(flags)
            .pool_sizes(pool_sizes)
            .max_sets(max_sets);
        let handle = unsafe { self.inner.create_descriptor_pool(&create_info, None) }?;
        Ok(DescriptorPool {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_query_pool(
        self: &Arc<Self>,
        query_type: QueryType,
        query_count: u32,
        pipeline_statistics: QueryPipelineStatisticFlags,
    ) -> VkResult<QueryPool> {
        let create_info = QueryPoolCreateInfo::builder()
            .query_type(query_type)
            .query_count(query_count)
            .pipeline_statistics(pipeline_statistics);
        let handle = unsafe { self.inner.create_query_pool(&create_info, None) }?;
        Ok(QueryPool {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_pipeline_layout(
        self: &Arc<Self>,
        flags: PipelineLayoutCreateFlags,
        set_layouts: &[&DescriptorSetLayout],
        push_constant_ranges: &[PushConstantRange],
    ) -> VkResult<PipelineLayout> {
        let set_layouts: Vec<_> = set_layouts
            .iter()
            .map(|set_layout| set_layout.handle)
            .collect();
        let create_info = PipelineLayoutCreateInfo::builder()
            .flags(flags)
            .set_layouts(&set_layouts)
            .push_constant_ranges(push_constant_ranges);
        let handle = unsafe { self.inner.create_pipeline_layout(&create_info, None) }?;
        Ok(PipelineLayout {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_buffer(
        self: &Arc<Self>,
        create_info: &BufferCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> vk_mem::Result<(Buffer, AllocationInfo)> {
        Buffer::new(self.clone(), create_info, allocation_info)
    }

    pub fn create_image(
        self: &Arc<Self>,
        create_info: &ImageCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> Result<(Image, AllocationInfo), Error> {
        Ok(Image::new(self.clone(), create_info, allocation_info)?)
    }

    pub fn create_image_view(
        self: &Arc<Self>,
        flags: ImageViewCreateFlags,
        image: &Image,
        view_type: ImageViewType,
        format: Format,
        components: ComponentMapping,
        subresource_range: ImageSubresourceRange,
    ) -> VkResult<ImageView> {
        let create_info = ImageViewCreateInfo::builder()
            .flags(flags)
            .image(image.handle)
            .view_type(view_type)
            .format(format)
            .components(components)
            .subresource_range(subresource_range);
        let handle = unsafe { self.inner.create_image_view(&create_info, None) }?;
        Ok(ImageView {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_render_pass(
        self: &Arc<Self>,
        create_info: &RenderPassCreateInfo,
    ) -> VkResult<RenderPass> {
        let handle = unsafe { self.inner.create_render_pass(create_info, None) }?;
        Ok(RenderPass {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_shader_module(
        self: &Arc<Self>,
        flags: ShaderModuleCreateFlags,
        code: &[u32],
    ) -> VkResult<ShaderModule> {
        let create_info = ShaderModuleCreateInfo::builder().flags(flags).code(code);
        let handle = unsafe { self.inner.create_shader_module(&create_info, None) }?;
        Ok(ShaderModule {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_sampler(self: &Arc<Self>, create_info: &SamplerCreateInfo) -> VkResult<Sampler> {
        let handle = unsafe { self.inner.create_sampler(create_info, None) }?;
        Ok(Sampler {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_pipeline_cache(
        self: &Arc<Self>,
        flags: PipelineCacheCreateFlags,
        initial_data: Option<&[u8]>,
    ) -> VkResult<PipelineCache> {
        let create_info = if let Some(initial_data) = initial_data {
            PipelineCacheCreateInfo::builder().initial_data(initial_data)
        } else {
            PipelineCacheCreateInfo::builder()
        }
        .flags(flags);
        let handle = unsafe { self.inner.create_pipeline_cache(&create_info, None) }?;
        Ok(PipelineCache {
            handle,
            device: self.clone(),
        })
    }

    pub fn create_vec<T: Copy>(
        self: &Arc<Self>,
        capacity: usize,
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
    ) -> Result<GpuVec<T>, Error> {
        Ok(GpuVec::new(
            self.clone(),
            capacity,
            buffer_usage,
            memory_usage,
        )?)
    }

    pub fn create_variable<T: Copy>(
        self: &Arc<Self>,
        buffer_usage: BufferUsageFlags,
        memory_usage: MemoryUsage,
        value: T,
    ) -> Result<GpuVariable<T>, Error> {
        Ok(GpuVariable::new(
            self.clone(),
            buffer_usage,
            memory_usage,
            value,
        )?)
    }

    pub fn update_descriptor_sets(
        self: &Arc<Self>,
        descriptor_writes: &[WriteDescriptorSet],
        descriptor_copies: &[CopyDescriptorSet],
    ) {
        unsafe {
            self.inner
                .update_descriptor_sets(&descriptor_writes, &descriptor_copies)
        }
    }

    pub fn wait_idle(&self) -> VkResult<()> {
        unsafe { self.inner.device_wait_idle() }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.allocator.destroy();
        unsafe { self.inner.destroy_device(None) }
    }
}

pub struct DeviceConfiguration {
    pub extension_names: Vec<&'static CStr>,
    pub features: PhysicalDeviceFeatures,
    pub main_queue_family_index: u32,
    pub transfer_queue_family_index: Option<u32>,
}

pub(super) struct DeviceQueue {
    pub(super) handle: Queue,
    family_index: u32,
}
