use std::sync::Arc;

use ash::vk::{self, Extent2D, ImageCreateInfo};
use vk_mem::{Allocation, AllocationCreateInfo, AllocationInfo};

use super::Device;

pub struct Image {
    pub(super) handle: vk::Image,
    allocation: Allocation,
    device: Arc<Device>,
}

impl Image {
    pub fn new(
        device: Arc<Device>,
        create_info: &ImageCreateInfo,
        allocation_info: &AllocationCreateInfo,
    ) -> vk_mem::Result<(Self, AllocationInfo)> {
        let (handle, allocation, allocation_info) = device
            .allocator
            .create_image(create_info, allocation_info)?;
        Ok((
            Self {
                handle,
                allocation,
                device,
            },
            allocation_info,
        ))
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        self.device
            .allocator
            .destroy_image(self.handle, &self.allocation)
            .unwrap()
    }
}

pub fn mip_levels_for_extent(extent: Extent2D) -> u32 {
    32 - extent.width.max(extent.height).leading_zeros()
}
