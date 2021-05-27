use std::{cell::RefCell, ffi::CString, fmt::Debug, io::stdout, sync::Arc, time::Instant};

use crate::vulkan::{choose_present_mode, choose_surface_format, DeviceCandidate};
use crate::winit::{run, Event, EventLoop, UserEvent};
use ::vulkan::{ApiResult, Device, Extent2D, Instance, Surface, Swapchain, SwapchainConfiguration};
use ::winit::{
    dpi::{PhysicalPosition, PhysicalSize, Position},
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, VirtualKeyCode, WindowEvent},
    window::{Fullscreen, Window, WindowBuilder},
};
use actor::{mailbox, Address, ControlFlow};
use ash_window::{create_surface, enumerate_required_extensions};
use context::Context;
use debug::DebugWindows;
use error::Error;
use interop::{ApplicationInfo, WorldFormat, WorldFormatReq};
use log::{error, LevelFilter, Log};
use log::{info, Level};
use nalgebra::{RealField, Translation3, UnitQuaternion, Vector3};
use net::Server;
use renderer::Renderer;
use resolver::Resolver;
use scene::{ControlState, Object, Transform};
use semver::{Compat, Version, VersionReq};
use serde_json::to_writer;
use server::{Connection, Push, Request, Service, PROTOCOL};
use structopt::StructOpt;
use tokio::{runtime::Runtime, spawn, task::JoinHandle};
use util::{handle::HandleFlow, iterator::MaxOkFilterMap};

mod context;
mod cull;
mod debug;
mod depth;
mod egui;
mod error;
mod frame;
mod renderer;
mod resolver;
mod scene;
mod shaders;
mod view;
mod vulkan;
mod winit;

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
    _server: Option<Server<Service>>,
    connection: Connection<Request>,
    windows: DebugWindows,
    winit: Address<crate::winit::Request>,
}

impl Application {
    async fn new(
        window: Window,
        winit: Address<crate::winit::Request>,
        address: Address<ApplicationMessage>,
        resolver: Resolver,
    ) -> Result<Self, Error> {
        let (connection, mut mailbox, mut _server) = resolver.resolve().await?;
        {
            let address = address.clone();
            spawn(async move {
                while let Some(message) = mailbox.recv().await {
                    if let Err(error) = address.send(ApplicationMessage::Push(message)) {
                        error!("{}", error);
                        break;
                    }
                }
            });
        }
        if let Some(server) = &mut _server {
            server.open(&"[::]:0".parse().unwrap()).unwrap();
        }
        let version = Version::parse(env!("CARGO_PKG_VERSION"))?;
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
        address
            .send(ApplicationMessage::Render)
            .map_err(Error::Send)?;
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
            _server,
            connection,
            windows: DebugWindows::default(),
            winit,
        })
    }

    fn spawn(
        event_loop: &EventLoop,
        winit: Address<crate::winit::Request>,
        resolver: Resolver,
    ) -> Result<(Address<Event>, JoinHandle<()>), Error> {
        let (mut mailbox, address) = mailbox();
        log::set_boxed_logger(Box::new(ApplicationLogger {
            address: address.clone(),
            secondary: env_logger::builder().build(),
        }))
        .unwrap();
        log::set_max_level(LevelFilter::Warn);
        let window = WindowBuilder::new()
            .with_title(format!("WoSim v{}", env!("CARGO_PKG_VERSION")))
            .build(event_loop)?;
        let handle = {
            let address = address.clone();
            spawn(async move {
                let mut application =
                    match Self::new(window, winit, address.clone(), resolver).await {
                        Ok(application) => application,
                        Err(error) => {
                            error!("Task failed: {:?}", error);
                            return;
                        }
                    };
                while let Some(message) = mailbox.recv().await {
                    match application.handle(message) {
                        Ok(ControlFlow::Continue) => {}
                        Ok(ControlFlow::Stop) => return,
                        Err(error) => {
                            error!("Task failed: {:?}", error);
                            return;
                        }
                    }
                }
            })
        };
        Ok((
            address.filter_map(|event| match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => Some(ApplicationMessage::ExitRequested),
                    event => Some(ApplicationMessage::WindowEvent(event)),
                },
                Event::DeviceEvent { device_id, event } => {
                    Some(ApplicationMessage::DeviceEvent(device_id, event))
                }
                Event::UserEvent(UserEvent::ScaleFactorChanged {
                    new_inner_size,
                    scale_factor,
                    ..
                }) => Some(ApplicationMessage::ScaleFactorChanged(
                    scale_factor,
                    new_inner_size,
                )),
                _ => None,
            }),
            handle,
        ))
    }

    fn handle(&mut self, message: ApplicationMessage) -> Result<ControlFlow, Error> {
        if let ApplicationMessage::ExitRequested = message {
            self.winit
                .send(crate::winit::Request::Exit)
                .map_err(Error::Send)?;
            return Ok(ControlFlow::Stop);
        }
        if self.handle_early_mouse_grab(&message).is_handled() {
            return Ok(ControlFlow::Continue);
        }
        if self.context.egui.handle_event(&message).is_handled() {
            return Ok(ControlFlow::Continue);
        }
        match message {
            ApplicationMessage::Push(push) => match push {
                Push::Positions(positions) => {
                    self.context.scene.clear();
                    for pos in positions {
                        self.context.scene.insert_object(Object {
                            model: self.context.cube_model,
                            transform: Transform {
                                translation: Vector3::new(pos.x, pos.y, pos.z),
                                scale: Vector3::new(0.3, 0.3, 0.3),
                                rotation: UnitQuaternion::identity(),
                            },
                        });
                    }
                }
            },
            ApplicationMessage::Disconnected => {
                info!("Disconnected from server");
                return Ok(ControlFlow::Stop);
            }
            ApplicationMessage::Log(level, target, args) => {
                self.context.debug.log(level, target, args);
            }
            ApplicationMessage::Render => {
                self.render()?;
            }
            ApplicationMessage::WindowEvent(event) => match event {
                WindowEvent::Resized(_) => {
                    self.recreate_renderer()?;
                }
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
                                    self.windows.information = !self.windows.information;
                                }
                            }
                            VirtualKeyCode::F2 => {
                                if input.state == ElementState::Pressed {
                                    self.windows.frame_times = !self.windows.frame_times;
                                }
                            }
                            VirtualKeyCode::F3 => {
                                if input.state == ElementState::Pressed {
                                    self.windows.log = !self.windows.log;
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
            ApplicationMessage::DeviceEvent(_, event) => {
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
            _ => {}
        }
        Ok(ControlFlow::Continue)
    }

    fn render(&mut self) -> Result<(), Error> {
        self.update();
        self.context.debug.begin_frame();
        let ctx = self.context.egui.begin();
        self.context
            .debug
            .render(&ctx, &mut self.windows, &self.connection);
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
        self.address
            .send(ApplicationMessage::Render)
            .map_err(Error::Send)?;
        Ok(())
    }

    fn handle_early_mouse_grab(&mut self, message: &ApplicationMessage) -> HandleFlow {
        if self.grab {
            if let ApplicationMessage::WindowEvent(WindowEvent::CursorMoved { .. }) = message {
                return HandleFlow::Handled;
            }
        }
        HandleFlow::Unhandled
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

#[derive(Debug)]
pub enum ApplicationMessage {
    Push(Push),
    Render,
    Disconnected,
    Log(Level, String, String),
    ScaleFactorChanged(f64, PhysicalSize<u32>),
    ExitRequested,
    WindowEvent(WindowEvent<'static>),
    DeviceEvent(DeviceId, DeviceEvent),
}

struct ApplicationLogger {
    address: Address<ApplicationMessage>,
    secondary: env_logger::Logger,
}

thread_local! {static INSIDE_LOG: RefCell<bool> = RefCell::new(false)}

impl Log for ApplicationLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if INSIDE_LOG.with(|i| i.replace(true)) {
            return;
        }
        let _ = self.address.send(ApplicationMessage::Log(
            record.level(),
            record.target().to_owned(),
            format!("{}", record.args()),
        ));
        self.secondary.log(record);
        INSIDE_LOG.with(|i| i.replace(false));
    }

    fn flush(&self) {
        self.secondary.flush();
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

#[derive(StructOpt)]
enum Command {
    Join {
        hostname: String,
        port: u16,
        token: String,
        #[structopt(long)]
        skip_verification: bool,
    },
    Play,
    Info,
    Create,
}

impl Command {
    fn run(self) -> Result<(), Error> {
        match self {
            Command::Join {
                hostname,
                port,
                token,
                skip_verification,
            } => run(
                Runtime::new()?,
                Self::spawn(Resolver::Remote {
                    hostname,
                    port,
                    token,
                    skip_verification,
                }),
            ),
            Command::Create => run(Runtime::new()?, Self::spawn(Resolver::Create)),
            Command::Play => run(Runtime::new()?, Self::spawn(Resolver::Open)),
            Command::Info => Ok(to_writer(
                stdout(),
                &ApplicationInfo {
                    name: format!("WoSim v{}", env!("CARGO_PKG_VERSION"),),
                    format: WorldFormat {
                        base: "mainline".to_owned(),
                        version: Version::new(0, 1, 0),
                    },
                    format_req: WorldFormatReq {
                        base: "mainline".to_owned(),
                        version: VersionReq::parse_compat(
                            env!("CARGO_PKG_VERSION"),
                            Compat::Cargo,
                        )?,
                    },
                    protocol: PROTOCOL.to_owned(),
                },
            )?),
        }
    }

    fn spawn(
        resolver: Resolver,
    ) -> impl FnOnce(
        &EventLoop,
        Address<winit::Request>,
    ) -> Result<(Address<Event>, JoinHandle<()>), Error> {
        move |event_loop, winit| Application::spawn(event_loop, winit, resolver)
    }
}

#[cfg(not(target_os = "macos"))]
fn setup_env() {}

#[cfg(target_os = "macos")]
fn setup_env() {
    set_var("MVK_CONFIG_FULL_IMAGE_VIEW_SWIZZLE", "1");
}

fn main() -> Result<(), Error> {
    setup_env();
    Command::from_args().run()
}
