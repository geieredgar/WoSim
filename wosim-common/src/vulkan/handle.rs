use ash::{
    version::DeviceV1_0,
    vk::{CommandBuffer, CommandPool, Fence, ImageView, Semaphore},
    Device,
};

pub trait Handle: Copy {
    /// # Safety
    unsafe fn destroy(self, device: &Device);
}

impl Handle for Fence {
    unsafe fn destroy(self, device: &Device) {
        device.destroy_fence(self, None)
    }
}

impl Handle for CommandPool {
    unsafe fn destroy(self, device: &Device) {
        device.destroy_command_pool(self, None)
    }
}

impl Handle for CommandBuffer {
    unsafe fn destroy(self, _device: &Device) {}
}

impl Handle for Semaphore {
    unsafe fn destroy(self, device: &Device) {
        device.destroy_semaphore(self, None)
    }
}

impl Handle for ImageView {
    unsafe fn destroy(self, device: &Device) {
        device.destroy_image_view(self, None)
    }
}

pub trait HandleWrapper {
    type Handle;
}

pub trait DerefHandle: Handle {}

impl DerefHandle for CommandBuffer {}
impl DerefHandle for Semaphore {}
