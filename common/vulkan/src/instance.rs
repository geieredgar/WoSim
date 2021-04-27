use std::{
    ffi::{CStr, CString},
    sync::Arc,
};

use ash::{
    prelude::VkResult,
    version::{EntryV1_0, InstanceV1_0},
    vk::{make_version, ApplicationInfo, InstanceCreateInfo, SurfaceKHR},
    Entry,
};

use crate::to_cstr;

use super::{Error, PhysicalDevice, Surface, Version};

pub struct Instance {
    pub(super) inner: ash::Instance,
    pub(super) physical_device_properties_2_support: bool,
    pub(super) entry: Entry,
}

impl Instance {
    pub fn new(
        application_name: &CStr,
        application_version: Version,
        extension_names: Vec<&CStr>,
    ) -> Result<Self, Error> {
        let entry = unsafe { Entry::new() }?;
        let layer_names = if cfg!(debug_assertions) {
            vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()]
        } else {
            vec![]
        };
        let mut physical_device_properties_2_support = false;
        for properties in entry.enumerate_instance_extension_properties()? {
            if unsafe { to_cstr(&properties.extension_name) }
                == ash::vk::KhrGetPhysicalDeviceProperties2Fn::name()
                && properties.spec_version >= 2
            {
                physical_device_properties_2_support = true;
            }
        }
        let application_info = ApplicationInfo::builder()
            .api_version(make_version(1, 2, 0))
            .application_name(application_name)
            .application_version(application_version.into());
        let extension_names_ptr: Vec<_> = extension_names.iter().map(|c| c.as_ptr()).collect();
        let layer_names_ptr: Vec<_> = layer_names.iter().map(|c| c.as_ptr()).collect();
        let create_info = InstanceCreateInfo::builder()
            .enabled_layer_names(&layer_names_ptr)
            .enabled_extension_names(&extension_names_ptr)
            .application_info(&application_info)
            .build();
        let inner = unsafe { entry.create_instance(&create_info, None) }?;
        Ok(Self {
            inner,
            physical_device_properties_2_support,
            entry,
        })
    }

    pub fn create_surface<F: Fn(&Entry, &ash::Instance) -> VkResult<SurfaceKHR>>(
        self: &Arc<Self>,
        create_handle: F,
    ) -> VkResult<Surface> {
        let handle = create_handle(&self.entry, &self.inner)?;
        Ok(Surface::new(self.clone(), handle))
    }

    pub fn physical_devices(self: &Arc<Self>) -> VkResult<Vec<PhysicalDevice>> {
        Ok(unsafe { self.inner.enumerate_physical_devices() }?
            .into_iter()
            .map(|handle| PhysicalDevice::new(self.clone(), handle))
            .collect())
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe { self.inner.destroy_instance(None) }
    }
}
