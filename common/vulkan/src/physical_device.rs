use std::{cmp::Ordering, sync::Arc};

use ash::{
    prelude::VkResult,
    version::{InstanceV1_0, InstanceV1_1},
    vk::{
        self, make_version, ExtensionProperties, Format, FormatProperties, PhysicalDeviceFeatures2,
        PhysicalDevicePortabilitySubsetFeaturesKHR, PhysicalDevicePortabilitySubsetPropertiesKHR,
        PhysicalDeviceProperties2, PhysicalDeviceType, PhysicalDeviceVulkan12Features,
        PresentModeKHR, QueueFamilyProperties, SurfaceCapabilitiesKHR, SurfaceFormatKHR,
    },
};
use vk::{DeviceCreateInfo, DeviceQueueCreateInfo};

use super::{Device, DeviceConfiguration, Error, Instance, Surface};

#[derive(Clone)]
pub struct PhysicalDevice {
    pub(super) instance: Arc<Instance>,
    pub(super) handle: vk::PhysicalDevice,
}

impl PhysicalDevice {
    pub(super) fn new(instance: Arc<Instance>, handle: vk::PhysicalDevice) -> Self {
        Self { instance, handle }
    }

    pub fn features(&self) -> PhysicalDeviceFeatures {
        let mut features = PhysicalDeviceFeatures::default();
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
        let mut properties = PhysicalDeviceProperties::default();
        if !self.instance.physical_device_properties_2_support {
            properties.vulkan_10.properties = unsafe {
                self.instance
                    .inner
                    .get_physical_device_properties(self.handle)
            };
            if properties.vulkan_10.properties.api_version < make_version(1, 2, 0) {
                return properties;
            }
        }
        unsafe {
            self.instance
                .inner
                .get_physical_device_properties2(self.handle, properties.chain())
        };
        properties
    }

    pub fn format_properties(&self, format: Format) -> FormatProperties {
        unsafe {
            self.instance
                .inner
                .get_physical_device_format_properties(self.handle, format)
        }
    }

    pub fn create(self, mut configuration: DeviceConfiguration) -> Result<Device, Error> {
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
        Ok(Device::new(self, device, configuration)?)
    }
}

#[derive(Default, Clone)]
pub struct PhysicalDeviceFeatures {
    pub vulkan_10: PhysicalDeviceFeatures2,
    pub vulkan_12: PhysicalDeviceVulkan12Features,
    pub portability_subset: PhysicalDevicePortabilitySubsetFeaturesKHR,
}

impl PhysicalDeviceFeatures {
    fn chain(&mut self) -> &mut PhysicalDeviceFeatures2 {
        self.vulkan_10.p_next =
            &mut self.vulkan_12 as *mut PhysicalDeviceVulkan12Features as *mut _;
        self.vulkan_12.p_next = &mut self.portability_subset
            as *mut PhysicalDevicePortabilitySubsetFeaturesKHR
            as *mut _;
        &mut self.vulkan_10
    }
}

#[derive(Default, Clone)]
pub struct PhysicalDeviceProperties {
    pub vulkan_10: PhysicalDeviceProperties2,
    pub portability_subset: PhysicalDevicePortabilitySubsetPropertiesKHR,
}

impl PhysicalDeviceProperties {
    fn chain(&mut self) -> &mut PhysicalDeviceProperties2 {
        self.vulkan_10.p_next = &mut self.portability_subset
            as *mut PhysicalDevicePortabilitySubsetPropertiesKHR
            as *mut _;
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
