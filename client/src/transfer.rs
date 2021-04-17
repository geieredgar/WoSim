use std::sync::Arc;

use vulkan::{Device, TransferBuffer, TransferPool};

use crate::{
    context::{Context, ContextUpdate},
    renderer::Renderer,
};

pub struct Transfers {
    _pool: TransferPool,
    pending: Vec<(TransferBuffer, Box<dyn ContextUpdate>)>,
}

impl Transfers {
    pub fn new(device: &Arc<Device>) -> Self {
        Self {
            _pool: device.create_transfer_pool(),
            pending: Vec::new(),
        }
    }

    pub fn _push<
        F: FnOnce(&mut TransferBuffer) -> Result<T, vulkan::Error>,
        T: 'static + ContextUpdate,
    >(
        &mut self,
        f: F,
    ) -> Result<(), vulkan::Error> {
        let mut buffer = self._pool.allocate()?;
        let update = Box::new(f(&mut buffer)?);
        buffer.submit()?;
        self.pending.push((buffer, update));
        Ok(())
    }

    pub fn poll(
        &mut self,
        device: &Arc<Device>,
        context: &mut Context,
        renderer: &mut Renderer,
    ) -> Result<(), vulkan::Error> {
        let mut pending = Vec::new();
        for (buffer, update) in self.pending.drain(..) {
            if buffer.ready()? {
                renderer.update(update.apply(device, context)?);
            } else {
                pending.push((buffer, update))
            }
        }
        Ok(())
    }
}
