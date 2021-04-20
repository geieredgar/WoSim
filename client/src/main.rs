use std::{ffi::CString, sync::Arc, time::Instant};

use crate::vulkan::{choose_present_mode, choose_surface_format, DeviceCandidate};
use ::vulkan::{
    ApiResult, Device, Extent2D, Instance, Surface, Swapchain, SwapchainConfiguration, Version,
};
use ash_window::{create_surface, enumerate_required_extensions};
use common::iterator::MaxOkFilterMap;
use context::Context;
use error::Error;
use nalgebra::{RealField, Translation3, UnitQuaternion, Vector3};
use renderer::Renderer;
use scene::ControlState;
use winit::{
    event::{DeviceEvent, ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowBuilder},
};

mod context;
mod cull;
mod debug;
mod depth;
mod egui;
mod error;
mod frame;
mod renderer;
mod scene;
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
    control_state: ControlState,
    grab: bool,
    vsync: bool,
    last_update: Instant,
    time: f32,
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
            control_state: ControlState {
                forward: false,
                backward: false,
                left: false,
                right: false,
            },
            grab: false,
            vsync: true,
            last_update: Instant::now(),
            time: 0.0,
        })
    }

    fn process_event(&mut self, event: Event<()>) -> Result<ControlFlow, Error> {
        self.context.egui.handle_event(&event);
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(_) => {
                    self.recreate_renderer()?;
                }
                WindowEvent::CloseRequested => return Ok(ControlFlow::Exit),
                WindowEvent::KeyboardInput {
                    device_id: _,
                    input,
                    is_synthetic: _,
                } => {
                    if let Some(keycode) = input.virtual_keycode {
                        match keycode {
                            VirtualKeyCode::W => {
                                self.control_state.forward = input.state == ElementState::Pressed;
                            }

                            VirtualKeyCode::A => {
                                self.control_state.left = input.state == ElementState::Pressed;
                            }
                            VirtualKeyCode::S => {
                                self.control_state.backward = input.state == ElementState::Pressed;
                            }
                            VirtualKeyCode::D => {
                                self.control_state.right = input.state == ElementState::Pressed;
                            }
                            VirtualKeyCode::F1 => {
                                if input.state == ElementState::Pressed
                                    && self.context.egui.is_enabled()
                                {
                                    self.set_grab(false)?;
                                }
                            }
                            VirtualKeyCode::F9 => {
                                if input.state == ElementState::Pressed {
                                    self.vsync = !self.vsync;
                                    self.recreate_renderer()?;
                                }
                            }
                            VirtualKeyCode::F10 => {
                                if input.state == ElementState::Pressed {
                                    if self.window.fullscreen().is_some() {
                                        self.window.set_fullscreen(None);
                                    } else {
                                        self.window
                                            .set_fullscreen(Some(Fullscreen::Borderless(None)));
                                    }
                                }
                            }
                            VirtualKeyCode::Escape => return Ok(ControlFlow::Exit),
                            _ => {}
                        }
                    }
                }
                WindowEvent::MouseInput { button, .. } => {
                    if button == MouseButton::Left && !self.context.egui.is_enabled() {
                        self.set_grab(true)?;
                    };
                }
                WindowEvent::Focused(focus) => {
                    if !focus {
                        self.set_grab(false)?;
                    }
                }
                _ => {}
            },
            Event::DeviceEvent {
                device_id: _,
                event,
            } => {
                if self.grab {
                    if let DeviceEvent::MouseMotion { delta } = event {
                        self.context.scene.camera.yaw += -0.0008 * delta.0 as f32;
                        self.context.scene.camera.pitch += -0.0008 * delta.1 as f32;
                        self.context.scene.camera.pitch = self
                            .context
                            .scene
                            .camera
                            .pitch
                            .clamp(-f32::pi() / 2.0, f32::pi() / 2.0);
                    }
                }
            }
            Event::MainEventsCleared => {
                self.update();
                self.context.debug.begin_frame();
                if let Some(ctx) = self.context.egui.begin() {
                    self.context.debug.render(&ctx);
                    self.context.egui.end(&self.window)?;
                }
                let result = self.renderer.render(&self.device, &mut self.context);
                let (resize, timestamps) = match result {
                    Ok(result) => (result.suboptimal, result.timestamps),
                    Err(err) => match err {
                        Error::Vulkan(vulkan_err) => match vulkan_err {
                            ::vulkan::Error::ApiResult(result) => {
                                if result == ApiResult::ERROR_OUT_OF_DATE_KHR {
                                    (true, None)
                                } else {
                                    return Err(Error::Vulkan(vulkan_err));
                                }
                            }
                            _ => return Err(Error::Vulkan(vulkan_err)),
                        },
                        _ => return Err(err),
                    },
                };
                self.context.debug.end_frame(timestamps);
                if resize {
                    self.recreate_renderer()?;
                }
            }
            _ => {}
        }
        Ok(ControlFlow::Poll)
    }

    fn recreate_renderer(&mut self) -> Result<(), Error> {
        self.device.wait_idle()?;
        self.swapchain = Arc::new(create_swapchain(
            &self.device,
            &self.surface,
            &self.window,
            self.vsync,
            Some(&self.swapchain),
        )?);
        self.renderer = Renderer::new(&self.device, &self.context, self.swapchain.clone())?;
        Ok(())
    }

    fn update(&mut self) {
        let now = Instant::now();
        let duration = now.duration_since(self.last_update);
        self.last_update = Instant::now();
        let distance = duration.as_secs_f32() * 10.0;
        self.time += duration.as_secs_f32();
        let angle = duration.as_secs_f32() * f32::pi() * 1.0;
        let mut translation = Vector3::<f32>::zeros();
        if self.control_state.forward {
            translation.z -= distance;
        }
        if self.control_state.backward {
            translation.z += distance;
        }
        if self.control_state.left {
            translation.x -= distance;
        }
        if self.control_state.right {
            translation.x += distance;
        }
        let translation: Translation3<_> =
            (self.context.scene.camera.rotation() * translation).into();
        self.context.scene.camera.translation *= translation;
        if self.context.debug.rotate_cubes {
            let rotation = UnitQuaternion::from_euler_angles(angle, angle, angle);
            let mut i = 0;
            for x in -20..21 {
                for y in -20..21 {
                    for z in -20..21 {
                        let object = &mut self.context.scene.objects[i];
                        object.transform.rotation *= rotation;
                        object.transform.translation = Vector3::new(
                            x as f32 * 3.0 + (self.time * 1.5).sin(),
                            y as f32 * 3.0 + (self.time * 1.5).cos(),
                            z as f32 * 3.0,
                        );
                        i += 1;
                    }
                }
            }
        }
    }

    fn set_grab(&mut self, grab: bool) -> Result<(), Error> {
        if self.grab == grab {
            return Ok(());
        }
        self.grab = grab;
        if grab {
            self.window.set_cursor_visible(false);
            self.window.set_cursor_grab(true)?;
        } else {
            self.window.set_cursor_visible(true);
            self.window.set_cursor_grab(false)?;
        }
        Ok(())
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
