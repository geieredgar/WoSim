use std::sync::Arc;

use eyre::Error;
use vulkan::{Device, Format, Swapchain};

use crate::{context::Context, frame::Frame, view::View};

const FRAMES_IN_FLIGHT: usize = 2;

pub struct Renderer {
    frame_count: usize,
    frames: [Frame; FRAMES_IN_FLIGHT],
    view: View,
    first_render: bool,
}

pub struct RenderResult {
    pub suboptimal: bool,
    pub timestamps: Option<RenderTimestamps>,
    pub last_draw_count: u32,
}

pub enum RenderError {
    Error(Error),
    OutOfDate,
}

impl From<vulkan::ApiResult> for RenderError {
    fn from(result: vulkan::ApiResult) -> Self {
        match result {
            vulkan::ApiResult::ERROR_OUT_OF_DATE_KHR => Self::OutOfDate,
            result => Self::Error(result.into()),
        }
    }
}

impl From<Error> for RenderError {
    fn from(error: Error) -> Self {
        Self::Error(error)
    }
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
    ) -> eyre::Result<Self> {
        let view = View::new(device, context, swapchain)?;
        let frames = [Frame::new(device, context)?, Frame::new(device, context)?];
        Ok(Self {
            frame_count: 0,
            frames,
            view,
            first_render: true,
        })
    }

    pub fn render(
        &mut self,
        device: &Arc<Device>,
        context: &mut Context,
    ) -> Result<RenderResult, RenderError> {
        if self.first_render {
            self.frames[0].setup_view(device, &self.view);
            self.frames[1].setup_view(device, &self.view);
            self.view.setup(device, context)?;
            self.first_render = false;
        }
        let frame_index = self.frame_count % FRAMES_IN_FLIGHT;
        self.frame_count += 1;
        let result = self.frames[frame_index].render(device, context, &self.view)?;
        Ok(result)
    }

    pub fn recreate_view(
        &mut self,
        device: &Arc<Device>,
        context: &Context,
        swapchain: Arc<Swapchain>,
    ) -> eyre::Result<()> {
        self.view = View::new(device, context, swapchain)?;
        self.first_render = true;
        Ok(())
    }
}
