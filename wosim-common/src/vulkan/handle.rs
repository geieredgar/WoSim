use ash::{version::DeviceV1_0, vk::Fence, Device};

pub trait Handle: Copy {
    /// # Safety
    unsafe fn destroy(self, device: &Device);
}

impl Handle for Fence {
    unsafe fn destroy(self, device: &Device) {
        device.destroy_fence(self, None)
    }
}
