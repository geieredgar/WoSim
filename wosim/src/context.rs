use std::sync::Arc;

use wosim_common::vulkan::Device;

use crate::error::Error;

pub struct Context {}

impl Context {
    pub fn new(_device: &Arc<Device>) -> Result<Self, Error> {
        Ok(Self {})
    }
}
