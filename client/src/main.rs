use std::{
    ffi::CString, fmt::Debug, fs::read, fs::read_dir, future::Future, sync::Arc, time::Instant,
};

use crate::vulkan::{choose_present_mode, choose_surface_format, DeviceCandidate};
use crate::winit::run;
use ::vulkan::{
    ApiResult, Device, Extent2D, Instance, Surface, Swapchain, SwapchainConfiguration, Version,
};
use ::winit::{
    dpi::{PhysicalPosition, Position},
    event::{DeviceEvent, ElementState, Event, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    window::{Fullscreen, Window, WindowBuilder},
};
use actor::Address;
use ash_window::{create_surface, enumerate_required_extensions};
use context::Context;
use debug::DebugWindows;
use error::Error;
use log::error;
use log::info;
use nalgebra::{RealField, Translation3, Vector3};
use renderer::Renderer;
use scene::ControlState;
use server::{
    Certificate, ClientMessage, ResolveError, Resolver, ServerAddress, ServerMessage,
    SessionMessage, Token,
};
use tokio::{runtime::Runtime, spawn};
use util::iterator::MaxOkFilterMap;

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
mod winit;

pub use crate::winit::EventResult;

struct Application {
    address: Address<ApplicationMessage>,
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
    server: Option<Address<ServerMessage>>,
    resolver: Arc<Resolver>,
    windows: DebugWindows,
}

pub enum ApplicationMessage {
    Client(SessionMessage<(), ClientMessage>),
    Connected(Address<ServerMessage>),
    Connect { address: String, username: String },
    Disconnect,
}

impl winit::Application for Application {
    type Message = ApplicationMessage;
    type Error = Error;

    fn new(
        event_loop: &EventLoop<ApplicationMessage>,
        address: Address<ApplicationMessage>,
    ) -> Result<Self, Error> {
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
        let context = Context::new(
            address.clone(),
            &device,
            render_configuration,
            window.scale_factor() as f32,
        )?;
        let swapchain = Arc::new(create_swapchain(&device, &surface, &window, false, None)?);
        let renderer = Renderer::new(&device, &context, swapchain.clone())?;
        let certificates = read_dir("ca")?
            .filter_map(|entry| entry.ok().map(|entry| read(entry.path()).ok()).flatten())
            .filter_map(|pem| Certificate::from_pem(&pem).ok())
            .collect();
        Ok(Self {
            address,
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
            server: None,
            resolver: Arc::new(Resolver::new(certificates)),
            windows: DebugWindows::default(),
        })
    }

    fn handle_event(
        &mut self,
        event: Event<()>,
        _target: &EventLoopWindowTarget<ApplicationMessage>,
    ) -> Result<ControlFlow, Error> {
        if self.handle_early_mouse_grab(&event).is_handled() {
            return Ok(ControlFlow::Poll);
        }
        if self.context.egui.handle_event(&event).is_handled() {
            return Ok(ControlFlow::Poll);
        }
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
                                if input.state == ElementState::Pressed {
                                    self.windows.configuration = !self.windows.configuration;
                                }
                            }
                            VirtualKeyCode::F2 => {
                                if input.state == ElementState::Pressed {
                                    self.windows.information = !self.windows.information;
                                }
                            }
                            VirtualKeyCode::F3 => {
                                if input.state == ElementState::Pressed {
                                    self.windows.frame_times = !self.windows.frame_times;
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
                            VirtualKeyCode::Escape => self.set_grab(false)?,
                            _ => {}
                        }
                    }
                }
                WindowEvent::MouseInput { button, .. } => {
                    if button == MouseButton::Left {
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
                let ctx = self.context.egui.begin();
                self.context.debug.render(&ctx, &mut self.windows);
                self.context
                    .egui
                    .end(if self.grab { None } else { Some(&self.window) })?;
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

    fn handle_message(
        &mut self,
        message: ApplicationMessage,
        _target: &EventLoopWindowTarget<ApplicationMessage>,
    ) -> Result<ControlFlow, Error> {
        match message {
            ApplicationMessage::Client(message) => match message {
                SessionMessage::Connect(_) => {
                    info!("Connected to server");
                }
                SessionMessage::Message(_, _) => {}
                SessionMessage::Disconnect(_) => {
                    info!("Disconnected from server");
                    self.server = None;
                    self.context.debug.connection_status_changed(false);
                }
            },
            ApplicationMessage::Connected(server) => {
                self.server = Some(server);
                self.context.debug.connection_status_changed(true);
            }
            ApplicationMessage::Connect { address, username } => {
                spawn(report(connect_to_server(
                    self.address.clone(),
                    self.resolver.clone(),
                    address.clone(),
                    username.clone(),
                )));
            }
            ApplicationMessage::Disconnect => {
                self.server = None;
            }
        };
        Ok(ControlFlow::Poll)
    }
}

async fn connect_to_server(
    application: Address<ApplicationMessage>,
    resolver: Arc<Resolver>,
    address: String,
    name: String,
) -> Result<(), ResolveError> {
    let client = application.clone().map(ApplicationMessage::Client);
    let (_, server) = resolver
        .resolve(|_| client, ServerAddress::Remote(address), Token { name })
        .await?;
    application.send(ApplicationMessage::Connected(server));
    Ok(())
}

async fn report<E: Debug>(f: impl Future<Output = Result<(), E>>) {
    if let Err(error) = f.await {
        error!("Task failed: {:?}", error);
    }
}

impl Application {
    fn handle_early_mouse_grab(&mut self, event: &Event<()>) -> EventResult {
        if self.grab {
            if let Event::WindowEvent {
                event: WindowEvent::CursorMoved { .. },
                ..
            } = event
            {
                return EventResult::Handled;
            }
        }
        EventResult::Unhandled
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
            let size = self.window.inner_size();
            let position = PhysicalPosition {
                x: size.width as i32 / 2,
                y: size.height as i32 / 2,
            };
            self.window
                .set_cursor_position(Position::Physical(position))?;
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
    env_logger::init();
    run::<Application>(Runtime::new()?);
}
