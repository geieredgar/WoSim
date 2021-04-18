use std::{ffi::CString, sync::Arc};

use ash_window::{create_surface, enumerate_required_extensions};
use context::Context;
use error::Error;
use renderer::Renderer;
use vulkan::{choose_present_mode, choose_surface_format, DeviceCandidate};
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use wosim_common::{
    iterator::MaxOkFilterMap,
    vulkan::{
        ApiResult, Device, Extent2D, Instance, Surface, Swapchain, SwapchainConfiguration, Version,
    },
};

mod context;
mod debug;
mod egui;
mod error;
mod frame;
mod renderer;
mod shaders;
mod view;
mod vulkan;

struct Application {
    renderer: Renderer,
    swapchain: Arc<Swapchain>,
    context: Context,
    device: Arc<Device>,
    surface: Surface,
    window: Window,
}

impl Application {
    fn new(event_loop: &EventLoop<()>) -> Result<Self, Error> {
        let window = WindowBuilder::new()
            .with_title(format!("Wosim v{}", env!("CARGO_PKG_VERSION")))
            .build(event_loop)?;
        let version = Version {
            major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            patch: env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
        };
        let instance = Arc::new(Instance::new(
            &CString::new("wosim").unwrap(),
            version,
            enumerate_required_extensions(&window)?,
        )?);
        let surface = instance.create_surface(|entry, instance| unsafe {
            create_surface(entry, instance, &window, None)
        })?;
        let (device, render_configuration) = instance
            .physical_devices()?
            .into_iter()
            .max_ok_filter_map(|physical_device| DeviceCandidate::new(physical_device, &surface))?
            .ok_or(Error::NoSuitableDeviceFound)?
            .create()?;
        let device = Arc::new(device);
        let context = Context::new(&device, render_configuration, window.scale_factor() as f32)?;
        let swapchain = Arc::new(create_swapchain(&device, &surface, &window, false, None)?);
        let renderer = Renderer::new(&device, &context, swapchain.clone())?;
        Ok(Self {
            renderer,
            swapchain,
            context,
            device,
            surface,
            window,
        })
    }

    fn process_event(&mut self, event: Event<()>) -> Result<ControlFlow, Error> {
        self.context.egui.handle_event(&event);
        match event {
            Event::WindowEvent { event, .. } => {
                if event == winit::event::WindowEvent::CloseRequested {
                    return Ok(ControlFlow::Exit);
                }
            }
            Event::MainEventsCleared => {
                self.context.debug.begin_frame();
                if let Some(ctx) = self.context.egui.begin() {
                    self.context.debug.render(&ctx);
                    self.context.egui.end(&self.window)?;
                }
                let resize = match self.renderer.render(&self.device, &mut self.context) {
                    Ok(result) => result.suboptimal,
                    Err(err) => match err {
                        Error::Vulkan(vulkan_err) => match vulkan_err {
                            wosim_common::vulkan::Error::ApiResult(result) => {
                                if result == ApiResult::ERROR_OUT_OF_DATE_KHR {
                                    true
                                } else {
                                    return Err(Error::Vulkan(vulkan_err));
                                }
                            }
                            _ => return Err(Error::Vulkan(vulkan_err)),
                        },
                        _ => return Err(err),
                    },
                };
                self.context.debug.end_frame();
                if resize {
                    self.device.wait_idle()?;
                    self.swapchain = Arc::new(create_swapchain(
                        &self.device,
                        &self.surface,
                        &self.window,
                        true,
                        Some(&self.swapchain),
                    )?);
                    self.renderer =
                        Renderer::new(&self.device, &self.context, self.swapchain.clone())?;
                }
            }
            _ => {}
        }
        Ok(ControlFlow::Poll)
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        self.device.wait_idle().unwrap()
    }
}

fn create_swapchain(
    device: &Arc<Device>,
    surface: &Surface,
    window: &Window,
    disable_vsync: bool,
    previous: Option<&Swapchain>,
) -> Result<Swapchain, Error> {
    let extent = window.inner_size();
    let extent = Extent2D {
        width: extent.width,
        height: extent.height,
    };
    let surface_format = choose_surface_format(device.physical_device(), surface)?
        .ok_or(Error::NoSuitableSurfaceFormat)?;
    let present_mode = choose_present_mode(device.physical_device(), surface, disable_vsync)?
        .ok_or(Error::NoSuitablePresentMode)?;
    let configuration = SwapchainConfiguration {
        surface,
        previous,
        present_mode,
        surface_format,
        extent,
    };
    Ok(device.create_swapchain(configuration)?)
}

fn main() -> Result<(), Error> {
    let event_loop = EventLoop::new();
    let mut application = Application::new(&event_loop)?;
    event_loop.run(
        move |event, _, control_flow| match application.process_event(event) {
            Ok(flow) => *control_flow = flow,
            Err(error) => {
                println!("Error: {:?}", error);
                *control_flow = ControlFlow::Exit;
            }
        },
    );
}
