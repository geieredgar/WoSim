use std::sync::Arc;

use wosim_common::vulkan::{Device, Swapchain, SwapchainImage};

use crate::{context::Context, error::Error};

pub struct View {
    pub images: Vec<SwapchainImage>,
    pub swapchain: Arc<Swapchain>,
}

impl View {
    pub fn new(
        _device: &Arc<Device>,
        _context: &Context,
        swapchain: Arc<Swapchain>,
    ) -> Result<Self, Error> {
        let images = swapchain.images()?;
        Ok(Self { swapchain, images })
    }
}
