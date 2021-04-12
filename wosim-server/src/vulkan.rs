use std::cmp::Ordering;

use wosim_common::vulkan::{
    cmp_device_types, contains_extension, Device, DeviceConfiguration, DeviceFeatures,
    KhrPortabilitySubsetFn, PhysicalDevice, PhysicalDeviceProperties, QueueFamilyProperties,
    QueueFlags, VkResult,
};

pub struct DeviceCandidate {
    physical_device: PhysicalDevice,
    device_configuration: DeviceConfiguration,
    properties: PhysicalDeviceProperties,
}

impl DeviceCandidate {
    pub fn new(physical_device: PhysicalDevice) -> VkResult<Option<Self>> {
        let features = DeviceFeatures::default();
        let extensions = physical_device.extension_properties()?;
        let mut extension_names = Vec::new();
        if contains_extension(&extensions, KhrPortabilitySubsetFn::name()) {
            extension_names.push(KhrPortabilitySubsetFn::name());
        }
        let families = physical_device.queue_family_properties();
        let main_queue_family_index = match families
            .iter()
            .enumerate()
            .map(|(index, properties)| (index as u32, properties))
            .filter(|(_, properties)| properties.queue_flags.contains(QueueFlags::COMPUTE))
            .max_by_key(|(_, properties)| queue_family_priority(*properties))
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

fn queue_family_priority(properties: &QueueFamilyProperties) -> u32 {
    if properties.queue_flags.contains(QueueFlags::GRAPHICS) {
        0
    } else {
        1
    }
}
