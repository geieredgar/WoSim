use std::sync::Arc;

use vulkan::{Device, Format, Swapchain};

use crate::{context::Context, error::Error, frame::Frame, view::View};

const FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    frame_index: usize,
    frames: [Frame; FRAMES_IN_FLIGHT],
    view: View,
}

pub struct RenderResult {
    pub suboptimal: bool,
    pub timestamps: Option<RenderTimestamps>,
    pub last_draw_count: u32,
}

pub struct RenderTimestamps {
    pub begin: f64,
    pub end: f64,
}

pub struct RenderConfiguration {
    pub depth_format: Format,
    pub depth_pyramid_format: Format,
    pub timestamp_period: f64,
    pub use_draw_count: bool,
}

impl Renderer {
    pub fn new(
        device: &Arc<Device>,
        context: &Context,
        swapchain: Arc<Swapchain>,
    ) -> Result<Self, Error> {
        let view = View::new(device, context, swapchain)?;
        let frames = [
            Frame::new(device, context, &view)?,
            Frame::new(device, context, &view)?,
        ];
        Ok(Self {
            frame_index: 0,
            frames,
            view,
        })
    }

    pub fn render(
        &mut self,
        device: &Arc<Device>,
        context: &mut Context,
    ) -> Result<RenderResult, Error> {
        let frame_index = self.frame_index;
        self.frame_index = (frame_index + 1) % FRAMES_IN_FLIGHT;
        self.frames[frame_index].render(device, context, &self.view)
    }
}
