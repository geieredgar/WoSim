use std::sync::Arc;

use ash::{extensions::khr, vk::SurfaceKHR};

use super::Instance;

pub struct Surface {
    pub(super) inner: khr::Surface,
    pub(super) handle: SurfaceKHR,
    _instance: Arc<Instance>,
}

impl Surface {
    pub(super) fn new(instance: Arc<Instance>, inner: khr::Surface, handle: SurfaceKHR) -> Self {
        Self {
            inner,
            handle,
            _instance: instance,
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe { self.inner.destroy_surface(self.handle, None) }
    }
}
