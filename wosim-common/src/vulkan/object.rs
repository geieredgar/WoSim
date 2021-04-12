use std::sync::Arc;

use ash::vk;

use super::{Device, Handle};

pub struct Object<T: Handle> {
    pub(super) device: Arc<Device>,
    pub(super) handle: T,
}

impl<T: Handle> Drop for Object<T> {
    fn drop(&mut self) {
        self.device.destroy_handle(self.handle)
    }
}

pub type Fence = Object<vk::Fence>;
