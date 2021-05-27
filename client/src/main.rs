use std::collections::HashMap;
use std::sync::Mutex;
use std::{cell::RefCell, ffi::CString, fmt::Debug, io::stdout, sync::Arc, time::Instant};

use crate::vulkan::{choose_present_mode, choose_surface_format, DeviceCandidate};
use crate::winit::run;
use ::vulkan::{ApiResult, Device, Extent2D, Instance, Surface, Swapchain, SwapchainConfiguration};
use ::winit::event::Event;
use ::winit::event_loop::EventLoop;
use ::winit::{
    dpi::PhysicalPosition,
    event::{DeviceEvent, ElementState, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    window::{Fullscreen, Window, WindowBuilder},
};
use actor::Address;
use ash_window::{create_surface, enumerate_required_extensions};
use context::Context;
use debug::DebugWindows;
use error::Error;
use interop::{ApplicationInfo, WorldFormat, WorldFormatReq};
use log::Level;
use log::{error, LevelFilter, Log};
use nalgebra::{RealField, Translation3, UnitQuaternion, Vector3};
use net::Server;
use renderer::Renderer;
use resolver::Resolver;
use scene::{ControlState, Object, Transform};
use semver::{Compat, Version, VersionReq};
use serde_json::to_writer;
use server::{
    Connection, Player, Position, Push, Request, Service, Setup, Update, UpdateBatch, PROTOCOL,
};
use structopt::StructOpt;
use tokio::{runtime::Runtime, spawn};
use util::{handle::HandleFlow, iterator::MaxOkFilterMap};
use uuid::Uuid;

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
    last_server_update: Instant,
    time: f32,
    server: Option<Server<Service>>,
    connection: Connection<Request>,
    windows: DebugWindows,
    uuid: Uuid,
    other_players: HashMap<Uuid, (Player, usize)>,
}

impl Application {
    fn is_setup(&self) -> bool {
        !self.uuid.is_nil()
    }

    fn render(&mut self) -> Result<(), Error> {
        if self.is_setup() {
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
                self.recreate_swapchain()?;
            }
        }
        Ok(())
    }

    fn handle_early_mouse_grab(&mut self, event: &Event<()>) -> HandleFlow {
        if self.grab {
            if let Event::WindowEvent {
                event: WindowEvent::CursorMoved { .. },
                ..
            } = event
            {
                return HandleFlow::Handled;
            }
        }
        HandleFlow::Unhandled
    }

    fn recreate_swapchain(&mut self) -> Result<(), Error> {
        self.device.wait_idle()?;
        self.swapchain = Arc::new(create_swapchain(
            &self.device,
            &self.surface,
            &self.window,
            self.vsync,
            Some(&self.swapchain),
        )?);
        self.renderer
            .recreate_view(&self.device, &self.context, self.swapchain.clone())?;
        Ok(())
    }

    fn update(&mut self) {
        let now = Instant::now();
        let duration = now.duration_since(self.last_update);
        self.last_update = now;
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
        let duration = now.duration_since(self.last_server_update);
        if duration.as_millis() > 1000 / 30 && self.is_setup() {
            self.last_server_update = now;
            let _ = self
                .connection
                .asynchronous()
                .send(Request::UpdatePosition(Position {
                    x: self.context.scene.camera.translation.vector[0],
                    y: self.context.scene.camera.translation.vector[1],
                    z: self.context.scene.camera.translation.vector[2],
                }));
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
            let size = self.window.inner_size();
            let position = PhysicalPosition {
                x: size.width as i32 / 2,
                y: size.height as i32 / 2,
            };
            self.window
                .set_cursor_position(::winit::dpi::Position::Physical(position))?;
            self.window.set_cursor_visible(true);
            self.window.set_cursor_grab(false)?;
        }
        Ok(())
    }
}

impl winit::Application for Application {
    type Message = ApplicationMessage;

    type Error = Error;

    type Args = Resolver;

    fn new(
        event_loop: &EventLoop<Self::Message>,
        runtime: &Runtime,
        resolver: Resolver,
    ) -> Result<Self, Self::Error> {
        let proxy = Arc::new(Mutex::new(event_loop.create_proxy()));
        let address =
            Address::new(move |m| proxy.lock().unwrap().send_event(m).map_err(|e| e.into()));
        log::set_boxed_logger(Box::new(ApplicationLogger {
            address: address.clone(),
            secondary: env_logger::builder().build(),
        }))
        .unwrap();
        log::set_max_level(LevelFilter::Warn);
        let window = WindowBuilder::new()
            .with_title(format!("WoSim v{}", env!("CARGO_PKG_VERSION")))
            .build(event_loop)?;
        let (connection, mut mailbox, mut server) = runtime.block_on(resolver.resolve())?;
        spawn(async move {
            while let Some(message) = mailbox.recv().await {
                if let Err(error) = address.send(ApplicationMessage::Push(message)) {
                    error!("{}", error);
                    break;
                }
            }
        });
        if let Some(server) = &mut server {
            server.open().unwrap();
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
        let now = Instant::now();
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
            last_update: now,
            last_server_update: now,
            time: 0.0,
            server,
            connection,
            windows: DebugWindows::default(),
            uuid: Uuid::nil(),
            other_players: HashMap::new(),
        })
    }

    fn handle(
        &mut self,
        event: Event<Self::Message>,
        _runtime: &Runtime,
    ) -> Result<::winit::event_loop::ControlFlow, Self::Error> {
        match event.map_nonuser_event() {
            Ok(event) => {
                if self.handle_early_mouse_grab(&event).is_handled() {
                    return Ok(ControlFlow::Poll);
                }
                if self.context.egui.handle_event(&event).is_handled() {
                    return Ok(ControlFlow::Poll);
                }
                match event {
                    Event::NewEvents(_) => {}
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(_) => {
                            self.recreate_swapchain()?;
                        }
                        WindowEvent::KeyboardInput {
                            device_id: _,
                            input,
                            is_synthetic: _,
                        } => {
                            if let Some(keycode) = input.virtual_keycode {
                                match keycode {
                                    VirtualKeyCode::W => {
                                        self.control_state.forward =
                                            input.state == ElementState::Pressed;
                                    }

                                    VirtualKeyCode::A => {
                                        self.control_state.left =
                                            input.state == ElementState::Pressed;
                                    }
                                    VirtualKeyCode::S => {
                                        self.control_state.backward =
                                            input.state == ElementState::Pressed;
                                    }
                                    VirtualKeyCode::D => {
                                        self.control_state.right =
                                            input.state == ElementState::Pressed;
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
                                            self.recreate_swapchain()?;
                                        }
                                    }
                                    VirtualKeyCode::F10 => {
                                        if input.state == ElementState::Pressed {
                                            if self.window.fullscreen().is_some() {
                                                self.window.set_fullscreen(None);
                                            } else {
                                                self.window.set_fullscreen(Some(
                                                    Fullscreen::Borderless(None),
                                                ));
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
                        WindowEvent::CloseRequested => return Ok(ControlFlow::Exit),
                        _ => {}
                    },
                    Event::DeviceEvent { event, .. } => {
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
                    Event::MainEventsCleared => self.render()?,
                    _ => {}
                }
            }
            Err(event) => {
                if let Event::UserEvent(message) = event {
                    match message {
                        ApplicationMessage::Push(push) => match push {
                            Push::Setup(Setup(uuid, players, positions)) => {
                                self.uuid = uuid;
                                self.other_players.clear();
                                let mut static_group = Vec::new();
                                for pos in positions {
                                    static_group.push(Object {
                                        model: self.context.cube_model,
                                        transform: Transform {
                                            translation: Vector3::new(pos.x, pos.y, pos.z),
                                            scale: Vector3::new(0.3, 0.3, 0.3),
                                            rotation: UnitQuaternion::identity(),
                                        },
                                    });
                                }
                                let mut player_group = Vec::new();
                                for (uuid, player) in players {
                                    if self.uuid != uuid {
                                        self.other_players
                                            .insert(uuid, (player.clone(), player_group.len()));
                                        player_group.push(Object {
                                            model: self.context.cube_model,
                                            transform: Transform {
                                                translation: Vector3::new(
                                                    player.position.x,
                                                    player.position.y,
                                                    player.position.z,
                                                ),
                                                scale: Vector3::new(1.0, 1.0, 1.0),
                                                rotation: UnitQuaternion::identity(),
                                            },
                                        })
                                    } else {
                                        self.context.scene.camera.translation = Translation3::new(
                                            player.position.x,
                                            player.position.y,
                                            player.position.z,
                                        );
                                    }
                                }
                                self.context.scene.groups.clear();
                                self.context.scene.groups.push(static_group);
                                self.context.scene.groups.push(player_group);
                            }
                            Push::Updates(UpdateBatch(updates, after_index)) => {
                                for update in &updates[after_index..] {
                                    match update {
                                        Update::NewPlayer(uuid, player) => {
                                            let player_group = &mut self.context.scene.groups[1];
                                            self.other_players.insert(
                                                *uuid,
                                                (player.clone(), player_group.len()),
                                            );
                                            player_group.push(Object {
                                                model: self.context.cube_model,
                                                transform: Transform {
                                                    translation: Vector3::new(
                                                        player.position.x,
                                                        player.position.y,
                                                        player.position.z,
                                                    ),
                                                    scale: Vector3::new(1.0, 1.0, 1.0),
                                                    rotation: UnitQuaternion::identity(),
                                                },
                                            })
                                        }
                                        Update::PlayerPosition(uuid, pos) => {
                                            if self.uuid != *uuid {
                                                let player_group =
                                                    &mut self.context.scene.groups[1];
                                                let (player, index) =
                                                    self.other_players.get_mut(uuid).unwrap();
                                                player.position = *pos;
                                                player_group[*index].transform.translation =
                                                    Vector3::new(
                                                        player.position.x,
                                                        player.position.y,
                                                        player.position.z,
                                                    );
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        ApplicationMessage::Log(level, target, args) => {
                            self.context.debug.log(level, target, args);
                        }
                    }
                }
            }
        }

        Ok(ControlFlow::Poll)
    }

    fn shutdown(&mut self, runtime: &Runtime) {
        if let Some(server) = self.server.as_mut() {
            server.close();
            let _ = runtime.block_on(server.service().stop());
        }
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
    Log(Level, String, String),
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
    Play {
        token: String,
        #[structopt(long, short, default_value)]
        port: u16,
    },
    Info,
    Create {
        token: String,
        #[structopt(long, short, default_value)]
        port: u16,
    },
    #[cfg(debug_assertions)]
    Debug(DebugCommand),
}

#[derive(StructOpt)]
enum DebugCommand {
    Play {
        #[structopt(default_value)]
        port: u16,
    },
    Create {
        #[structopt(default_value)]
        port: u16,
    },
    Join {
        port: u16,
        #[structopt(long, short, default_value, default_value = "localhost")]
        hostname: String,
    },
}

impl Command {
    fn run(self) -> Result<(), Error> {
        match self {
            Command::Join {
                hostname,
                port,
                token,
                skip_verification,
            } => run::<Application>(
                Runtime::new()?,
                Resolver::Remote {
                    hostname,
                    port,
                    token,
                    skip_verification,
                },
            ),
            Command::Create { token, port } => {
                run::<Application>(Runtime::new()?, Resolver::Create { token, port })
            }
            Command::Play { token, port } => {
                run::<Application>(Runtime::new()?, Resolver::Open { token, port })
            }
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
            Command::Debug(command) => {
                let token = format!("{}#{}", Uuid::new_v4(), base64::encode("Debugger"));
                match command {
                    DebugCommand::Play { port } => {
                        run::<Application>(Runtime::new()?, Resolver::Open { token, port })
                    }
                    DebugCommand::Create { port } => {
                        std::fs::remove_file("world.db")?;
                        run::<Application>(Runtime::new()?, Resolver::Create { token, port })
                    }
                    DebugCommand::Join { hostname, port } => run::<Application>(
                        Runtime::new()?,
                        Resolver::Remote {
                            hostname,
                            port,
                            token,
                            skip_verification: true,
                        },
                    ),
                }
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn setup_env() {}

#[cfg(target_os = "macos")]
fn setup_env() {
    std::env::set_var("MVK_CONFIG_FULL_IMAGE_VIEW_SWIZZLE", "1");
}

fn main() -> Result<(), Error> {
    setup_env();
    Command::from_args().run()
}
