use std::{cmp::Ordering, ffi::CStr, sync::Arc};

use ash::{
    prelude::VkResult,
    version::{DeviceV1_0, InstanceV1_0, InstanceV1_1},
    vk::{
        self, CommandPoolCreateFlags, CommandPoolCreateInfo, ExtensionProperties, FenceCreateFlags,
        FenceCreateInfo, ImageViewCreateInfo, PhysicalDeviceFeatures2, PhysicalDeviceProperties,
        PhysicalDeviceType, PhysicalDeviceVulkan12Features, PresentModeKHR, Queue,
        QueueFamilyProperties, SemaphoreCreateInfo, SubmitInfo, SurfaceCapabilitiesKHR,
        SurfaceFormatKHR,
    },
};
use vk::{DeviceCreateInfo, DeviceQueueCreateInfo};

use super::{
    CommandPool, Fence, Handle, ImageView, Instance, Semaphore, Surface, Swapchain,
    SwapchainConfiguration,
};

#[derive(Clone)]
pub struct PhysicalDevice {
    pub(super) instance: Arc<Instance>,
    handle: vk::PhysicalDevice,
}

impl PhysicalDevice {
    pub(super) fn new(instance: Arc<Instance>, handle: vk::PhysicalDevice) -> Self {
        Self { instance, handle }
    }

    pub fn features(&self) -> DeviceFeatures {
        let mut features = DeviceFeatures::default();
        unsafe {
            self.instance
                .inner
                .get_physical_device_features2(self.handle, features.chain())
        };
        features
    }

    pub fn extension_properties(&self) -> VkResult<Vec<ExtensionProperties>> {
        unsafe {
            self.instance
                .inner
                .enumerate_device_extension_properties(self.handle)
        }
    }

    pub fn queue_family_properties(&self) -> Vec<QueueFamilyProperties> {
        unsafe {
            self.instance
                .inner
                .get_physical_device_queue_family_properties(self.handle)
        }
    }

    pub fn surface_support(&self, surface: &Surface, queue_family_index: u32) -> VkResult<bool> {
        unsafe {
            surface.inner.get_physical_device_surface_support(
                self.handle,
                queue_family_index,
                surface.handle,
            )
        }
    }

    pub fn surface_formats(&self, surface: &Surface) -> VkResult<Vec<SurfaceFormatKHR>> {
        unsafe {
            surface
                .inner
                .get_physical_device_surface_formats(self.handle, surface.handle)
        }
    }

    pub fn surface_present_modes(&self, surface: &Surface) -> VkResult<Vec<PresentModeKHR>> {
        unsafe {
            surface
                .inner
                .get_physical_device_surface_present_modes(self.handle, surface.handle)
        }
    }

    pub fn surface_capabilities(&self, surface: &Surface) -> VkResult<SurfaceCapabilitiesKHR> {
        unsafe {
            surface
                .inner
                .get_physical_device_surface_capabilities(self.handle, surface.handle)
        }
    }

    pub fn properties(&self) -> PhysicalDeviceProperties {
        unsafe {
            self.instance
                .inner
                .get_physical_device_properties(self.handle)
        }
    }

    pub fn create(self, mut configuration: DeviceConfiguration) -> VkResult<Device> {
        let queue_priorities = [1.0];
        let mut queue_create_infos = vec![DeviceQueueCreateInfo::builder()
            .queue_family_index(configuration.main_queue_family_index)
            .queue_priorities(&queue_priorities)
            .build()];
        if let Some(transfer_family_index) = configuration.transfer_queue_family_index {
            queue_create_infos.push(
                DeviceQueueCreateInfo::builder()
                    .queue_family_index(transfer_family_index)
                    .queue_priorities(&queue_priorities)
                    .build(),
            )
        }
        let extension_names_ptr: Vec<_> = configuration
            .extension_names
            .iter()
            .map(|c| c.as_ptr())
            .collect();
        let create_info = DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&extension_names_ptr)
            .push_next(configuration.features.chain());
        let device = unsafe {
            self.instance
                .inner
                .create_device(self.handle, &create_info, None)?
        };
        Ok(Device::new(self, device, configuration))
    }
}

pub struct Device {
    transfer_queue: Option<DeviceQueue>,
    pub(super) main_queue: DeviceQueue,
    pub(super) inner: ash::Device,
    physical_device: PhysicalDevice,
}

impl Device {
    fn new(
        physical_device: PhysicalDevice,
        inner: ash::Device,
        configuration: DeviceConfiguration,
    ) -> Device {
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
        Self {
            transfer_queue,
            main_queue,
            inner,
            physical_device,
        }
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

    pub fn transfer_submit(&self, submits: &[SubmitInfo], fence: &Fence) -> VkResult<()> {
        unsafe {
            self.inner.queue_submit(
                self.transfer_queue
                    .as_ref()
                    .unwrap_or(&self.main_queue)
                    .handle,
                submits,
                fence.handle,
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

    pub fn create_image_view(
        self: &Arc<Self>,
        create_info: &ImageViewCreateInfo,
    ) -> VkResult<ImageView> {
        let handle = unsafe { self.inner.create_image_view(&create_info, None) }?;
        Ok(ImageView {
            handle,
            device: self.clone(),
        })
    }

    pub fn wait_idle(&self) -> VkResult<()> {
        unsafe { self.inner.device_wait_idle() }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { self.inner.destroy_device(None) }
    }
}

pub struct DeviceConfiguration {
    pub extension_names: Vec<&'static CStr>,
    pub features: DeviceFeatures,
    pub main_queue_family_index: u32,
    pub transfer_queue_family_index: Option<u32>,
}

pub(super) struct DeviceQueue {
    pub(super) handle: Queue,
    family_index: u32,
}

#[derive(Default, Clone)]
pub struct DeviceFeatures {
    pub vulkan_10: PhysicalDeviceFeatures2,
    pub vulkan_12: PhysicalDeviceVulkan12Features,
}

impl DeviceFeatures {
    fn chain(&mut self) -> &mut PhysicalDeviceFeatures2 {
        self.vulkan_10.p_next =
            &mut self.vulkan_12 as *mut PhysicalDeviceVulkan12Features as *mut _;
        &mut self.vulkan_10
    }
}

pub fn cmp_device_types(a: PhysicalDeviceType, b: PhysicalDeviceType) -> Ordering {
    device_type_priority(a).cmp(&device_type_priority(b))
}

fn device_type_priority(device_type: PhysicalDeviceType) -> u32 {
    if device_type == PhysicalDeviceType::DISCRETE_GPU {
        5
    } else if device_type == PhysicalDeviceType::INTEGRATED_GPU {
        4
    } else if device_type == PhysicalDeviceType::CPU {
        3
    } else if device_type == PhysicalDeviceType::VIRTUAL_GPU {
        2
    } else if device_type == PhysicalDeviceType::OTHER {
        1
    } else {
        0
    }
}
