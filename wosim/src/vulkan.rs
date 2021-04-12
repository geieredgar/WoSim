use std::cmp::{Ordering, Reverse};

use wosim_common::vulkan::{
    cmp_device_types, contains_extension, ColorSpaceKHR, Device, DeviceConfiguration,
    DeviceFeatures, Format, KhrPortabilitySubsetFn, PhysicalDevice, PhysicalDeviceProperties,
    PresentModeKHR, QueueFlags, Surface, SurfaceFormatKHR, Swapchain, SwapchainConfiguration,
    VkResult,
};

pub struct DeviceCandidate {
    physical_device: PhysicalDevice,
    device_configuration: DeviceConfiguration,
    properties: PhysicalDeviceProperties,
}

impl DeviceCandidate {
    pub fn new(physical_device: PhysicalDevice, surface: &Surface) -> VkResult<Option<Self>> {
        if configure_swapchain(&physical_device, surface, true)?.is_none() {
            return Ok(None);
        };
        let features = DeviceFeatures::default();
        let extensions = physical_device.extension_properties()?;
        if !contains_extension(&extensions, Swapchain::extension_name()) {
            return Ok(None);
        }
        let mut extension_names = vec![Swapchain::extension_name()];
        if contains_extension(&extensions, KhrPortabilitySubsetFn::name()) {
            extension_names.push(KhrPortabilitySubsetFn::name());
        }
        let families = physical_device.queue_family_properties();
        let main_queue_family_index = match families
            .iter()
            .enumerate()
            .map(|(index, properties)| (index as u32, properties))
            .find(|(index, properties)| {
                match physical_device.surface_support(surface, *index) {
                    Ok(support) => {
                        if !support {
                            return false;
                        }
                    }
                    Err(_) => return false,
                }
                if !properties.queue_flags.contains(QueueFlags::GRAPHICS) {
                    return false;
                }
                properties.queue_flags.contains(QueueFlags::COMPUTE)
            })
            .map(|(index, _)| index as u32)
        {
            Some(index) => index,
            None => return Ok(None),
        };
        let transfer_queue_family_index = families
            .iter()
            .enumerate()
            .map(|(index, properties)| (index as u32, properties))
            .find(|(_, properties)| {
                properties.queue_flags.contains(QueueFlags::TRANSFER)
                    && !properties.queue_flags.contains(QueueFlags::GRAPHICS)
                    && !properties.queue_flags.contains(QueueFlags::COMPUTE)
            })
            .map(|(index, _)| index as u32);
        let device_configuration = DeviceConfiguration {
            extension_names,
            features,
            main_queue_family_index,
            transfer_queue_family_index,
        };
        let properties = physical_device.properties();
        Ok(Some(Self {
            physical_device,
            device_configuration,
            properties,
        }))
    }

    pub fn create(self) -> VkResult<Device> {
        self.physical_device.create(self.device_configuration)
    }
}

impl PartialEq for DeviceCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for DeviceCandidate {}

impl PartialOrd for DeviceCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DeviceCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        cmp_device_types(self.properties.device_type, other.properties.device_type)
    }
}

fn present_mode_priority(present_mode: PresentModeKHR, vsync: bool) -> usize {
    if present_mode == PresentModeKHR::IMMEDIATE && !vsync {
        3
    } else if present_mode == PresentModeKHR::MAILBOX {
        2
    } else if present_mode == PresentModeKHR::FIFO {
        1
    } else {
        0
    }
}

fn surface_format_priority(surface_format: SurfaceFormatKHR) -> usize {
    if surface_format.format == Format::B8G8R8A8_SRGB
        && surface_format.color_space == ColorSpaceKHR::SRGB_NONLINEAR
    {
        1
    } else {
        0
    }
}

pub fn configure_swapchain(
    physical_device: &PhysicalDevice,
    surface: &Surface,
    vsync: bool,
) -> VkResult<Option<SwapchainConfiguration>> {
    let surface_format = if let Some(surface_format) = physical_device
        .surface_formats(surface)?
        .into_iter()
        .min_by_key(|surface_format| Reverse(surface_format_priority(*surface_format)))
    {
        surface_format
    } else {
        return Ok(None);
    };
    let present_mode = if let Some(present_mode) = physical_device
        .surface_present_modes(surface)?
        .into_iter()
        .min_by_key(|present_mode| Reverse(present_mode_priority(*present_mode, vsync)))
    {
        present_mode
    } else {
        return Ok(None);
    };
    Ok(Some(SwapchainConfiguration {
        surface_format,
        present_mode,
    }))
}
