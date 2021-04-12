use std::{ffi::CString, sync::Arc};

use ash_window::{create_surface, enumerate_required_extensions};
use error::Error;
use vulkan::DeviceCandidate;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use wosim_common::{
    iterator::MaxOkFilterMap,
    vulkan::{Device, Instance, Surface, Version},
};

mod error;
mod vulkan;

struct Application {
    _device: Arc<Device>,
    _surface: Surface,
    _window: Window,
}

impl Application {
    fn process_event(&mut self, event: Event<()>) -> ControlFlow {
        if let Event::WindowEvent { event, .. } = event {
            if event == winit::event::WindowEvent::CloseRequested {
                return ControlFlow::Exit;
            }
        }
        ControlFlow::Poll
    }
}

fn main() -> Result<(), Error> {
    let version = Version {
        major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
        minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
        patch: env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
    };
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(format!("Wosim v{}", env!("CARGO_PKG_VERSION")))
        .build(&event_loop)?;
    let instance = Arc::new(Instance::new(
        &CString::new("wosim").unwrap(),
        version,
        enumerate_required_extensions(&window)?,
    )?);
    let surface = instance.create_surface(|entry, instance| unsafe {
        create_surface(entry, instance, &window, None)
    })?;
    let device = Arc::new(
        instance
            .physical_devices()?
            .into_iter()
            .max_ok_filter_map(|physical_device| DeviceCandidate::new(physical_device, &surface))?
            .ok_or(Error::NoSuitableDeviceFound)?
            .create()?,
    );
    let mut application = Application {
        _device: device,
        _surface: surface,
        _window: window,
    };
    event_loop.run(move |event, _, control_flow| *control_flow = application.process_event(event));
}
