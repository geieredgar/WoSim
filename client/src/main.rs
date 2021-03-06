use std::collections::HashMap;
use std::sync::Mutex;
use std::{cell::RefCell, ffi::CString, fmt::Debug, sync::Arc, time::Instant};

use crate::vulkan::{choose_present_mode, choose_surface_format, DeviceCandidate};
use ::vulkan::{Device, Extent2D, Instance, Surface, Swapchain, SwapchainConfiguration};
use ::winit::event::Event;
use ::winit::event_loop::EventLoop;
use ::winit::event_loop::EventLoopProxy;
use ::winit::{
    dpi::PhysicalPosition,
    event::{DeviceEvent, ElementState, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    window::{Fullscreen, Window, WindowBuilder},
};
use ash_window::{create_surface, enumerate_required_extensions};
use context::Context;
use debug::DebugWindows;
use eyre::{eyre, Context as EyreContext};
use log::Level;
use log::{error, Log};
use nalgebra::Rotation3;
use nalgebra::{RealField, Translation3, UnitQuaternion, Vector3};
use net::Server;
use renderer::RenderError;
use renderer::Renderer;
use resolver::Resolver;
use scene::{ControlState, Object, Transform};
use semver::Version;
use server::Orientation;
use server::SelfUpdate;
use server::{Connection, Player, Position, Push, Request, Service, Setup, Update, UpdateBatch};
use structopt::StructOpt;
use tokio::{runtime::Runtime, spawn, task::JoinHandle};
use util::{handle::HandleFlow, iterator::MaxOkFilterMap};
use uuid::Uuid;

mod context;
mod cull;
mod debug;
mod depth;
mod egui;
mod frame;
mod renderer;
mod resolver;
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
    last_server_update: Instant,
    time: f32,
    server: Option<Server<Service>>,
    connection: Connection<Request>,
    windows: DebugWindows,
    uuid: Uuid,
    other_players: HashMap<Uuid, (Player, usize)>,
    handle: Option<JoinHandle<()>>,
}

impl Application {
    async fn new(
        event_loop: &EventLoop<ApplicationMessage>,
        resolver: Resolver,
    ) -> eyre::Result<Self> {
        let env_logger = env_logger::builder().build();
        let filter = env_logger.filter();
        log::set_boxed_logger(Box::new(ApplicationLogger {
            proxy: Mutex::new(event_loop.create_proxy()),
            secondary: env_logger,
        }))
        .wrap_err("could not set logger")?;
        log::set_max_level(filter);
        let window = WindowBuilder::new()
            .with_title(format!("WoSim v{}", env!("CARGO_PKG_VERSION")))
            .build(event_loop)?;
        let (connection, mut mailbox, mut server) = resolver
            .resolve()
            .await
            .wrap_err("could not connect to server")?;
        let proxy = event_loop.create_proxy();
        let handle = spawn(async move {
            while let Some(message) = mailbox.recv().await {
                if let Err(error) = proxy.send_event(ApplicationMessage::Push(message)) {
                    error!("{:?}", error);
                    break;
                }
            }
            if let Err(error) = proxy.send_event(ApplicationMessage::Disconnected) {
                error!("{:?}", error);
            };
        });
        if let Some(server) = &mut server {
            server.open().wrap_err("could not open server")?;
        }
        let version = Version::parse(env!("CARGO_PKG_VERSION"))?;
        let instance = Arc::new(
            Instance::new(
                &CString::new("wosim").unwrap(),
                version,
                enumerate_required_extensions(&window)?,
            )
            .wrap_err("could not create instance")?,
        );
        let surface = instance.create_surface(|entry, instance| unsafe {
            create_surface(entry, instance, &window, None)
        })?;
        let (device, render_configuration) = instance
            .physical_devices()?
            .into_iter()
            .max_ok_filter_map(|physical_device| DeviceCandidate::new(physical_device, &surface))?
            .ok_or_else(|| eyre!("could not find suitable device"))?
            .create()
            .wrap_err("could not create device")?;
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
            handle: Some(handle),
        })
    }

    async fn handle(&mut self, event: Event<'_, ApplicationMessage>) -> eyre::Result<ControlFlow> {
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
                    Event::MainEventsCleared => self.render().await?,
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
                                for player in players {
                                    let uuid = Uuid::from_u128(player.uuid);
                                    if self.uuid != uuid {
                                        self.other_players
                                            .insert(uuid, (player, player_group.len()));
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
                                        Update::NewPlayer(player) => {
                                            let player_group = &mut self.context.scene.groups[1];
                                            self.other_players.insert(
                                                Uuid::from_u128(player.uuid),
                                                (*player, player_group.len()),
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
                                        Update::Player(uuid, pos, orientation) => {
                                            if self.uuid != *uuid {
                                                let player_group =
                                                    &mut self.context.scene.groups[1];
                                                let (player, index) =
                                                    self.other_players.get_mut(uuid).unwrap();
                                                player.position = *pos;
                                                player_group[*index].transform.rotation =
                                                    (Rotation3::from_axis_angle(
                                                        &Vector3::y_axis(),
                                                        orientation.yaw,
                                                    ) * Rotation3::from_axis_angle(
                                                        &Vector3::x_axis(),
                                                        orientation.pitch,
                                                    ) * Rotation3::from_axis_angle(
                                                        &Vector3::z_axis(),
                                                        orientation.roll,
                                                    ))
                                                    .into();
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
                        ApplicationMessage::Disconnected => {
                            return Ok(ControlFlow::Exit);
                        }
                    }
                }
            }
        }

        Ok(ControlFlow::Poll)
    }

    async fn shutdown(&mut self) -> eyre::Result<()> {
        self.connection.send(Request::Shutdown).await?;
        self.handle.take().unwrap().await?;
        if let Some(server) = self.server.as_mut() {
            server.service().stop().await;
            server.close();
        }
        Ok(())
    }

    fn is_setup(&self) -> bool {
        !self.uuid.is_nil()
    }

    async fn render(&mut self) -> eyre::Result<()> {
        if self.is_setup() {
            self.update().await;
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
                    RenderError::Error(error) => return Err(error),
                    RenderError::OutOfDate => (true, None),
                },
            };
            self.context.debug.end_frame(timestamps, &self.connection);
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

    fn recreate_swapchain(&mut self) -> eyre::Result<()> {
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

    async fn update(&mut self) {
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
                .send(Request::UpdateSelf(SelfUpdate(
                    Position {
                        x: self.context.scene.camera.translation.vector[0],
                        y: self.context.scene.camera.translation.vector[1],
                        z: self.context.scene.camera.translation.vector[2],
                    },
                    Orientation {
                        roll: self.context.scene.camera.roll,
                        pitch: self.context.scene.camera.pitch,
                        yaw: self.context.scene.camera.yaw,
                    },
                )))
                .await;
        }
    }

    fn set_grab(&mut self, grab: bool) -> eyre::Result<()> {
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

impl Drop for Application {
    fn drop(&mut self) {
        self.device.wait_idle().unwrap()
    }
}

#[derive(Debug)]
pub enum ApplicationMessage {
    Push(Push),
    Log(Level, String, String),
    Disconnected,
}

struct ApplicationLogger {
    proxy: Mutex<EventLoopProxy<ApplicationMessage>>,
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
        let _ = self
            .proxy
            .lock()
            .unwrap()
            .send_event(ApplicationMessage::Log(
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
) -> eyre::Result<Swapchain> {
    let extent = window.inner_size();
    let extent = Extent2D {
        width: extent.width,
        height: extent.height,
    };
    let surface_format = choose_surface_format(device.physical_device(), surface)?
        .ok_or_else(|| eyre!("could not find suitable surface format"))?;
    let present_mode = choose_present_mode(device.physical_device(), surface, disable_vsync)?
        .ok_or_else(|| eyre!("could not find suitable present mode"))?;
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
    Create {
        token: String,
        #[structopt(long, short, default_value)]
        port: u16,
    },
    #[cfg(debug_assertions)]
    Debug(DebugCommand),
}

#[cfg(debug_assertions)]
#[derive(StructOpt)]
enum DebugCommand {
    Play {
        #[structopt(default_value = "1996")]
        port: u16,
    },
    Create {
        #[structopt(default_value = "1996")]
        port: u16,
    },
    Join {
        #[structopt(default_value = "1996")]
        port: u16,
        #[structopt(long, short, default_value, default_value = "localhost")]
        hostname: String,
        #[structopt(long)]
        verify: bool,
    },
}

impl Command {
    fn run(self) -> eyre::Result<()> {
        match self {
            Command::Join {
                hostname,
                port,
                token,
                skip_verification,
            } => Runner::run(Resolver::Remote {
                hostname,
                port,
                token,
                skip_verification,
            }),
            Command::Create { token, port } => Runner::run(Resolver::Create { token, port }),
            Command::Play { token, port } => Runner::run(Resolver::Open { token, port }),
            #[cfg(debug_assertions)]
            Command::Debug(command) => {
                let token = format!("{}#{}", Uuid::new_v4(), base64::encode("Debugger"));
                match command {
                    DebugCommand::Play { port } => Runner::run(Resolver::Open { token, port }),
                    DebugCommand::Create { port } => {
                        let _ = std::fs::remove_file("world.db");
                        Runner::run(Resolver::Create { token, port })
                    }
                    DebugCommand::Join {
                        hostname,
                        port,
                        verify,
                    } => Runner::run(Resolver::Remote {
                        hostname,
                        port,
                        token,
                        skip_verification: !verify,
                    }),
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

fn main() -> eyre::Result<()> {
    stable_eyre::install()?;
    setup_env();
    Command::from_args().run()
}

struct Runner {
    application: Application,
    runtime: Runtime,
}

impl Runner {
    pub fn new(
        event_loop: &EventLoop<ApplicationMessage>,
        resolver: Resolver,
    ) -> eyre::Result<Self> {
        let runtime = Runtime::new()?;
        let _guard = runtime.enter();
        let application = runtime.block_on(Application::new(event_loop, resolver))?;
        Ok(Self {
            application,
            runtime,
        })
    }

    pub fn handle(&mut self, event: Event<ApplicationMessage>) -> eyre::Result<ControlFlow> {
        let _guard = self.runtime.enter();
        self.runtime.block_on(self.application.handle(event))
    }

    pub fn run(resolver: Resolver) -> eyre::Result<()> {
        let event_loop = EventLoop::with_user_event();
        let mut runner = Runner::new(&event_loop, resolver)?;
        event_loop.run(move |event, _, control_flow| match runner.handle(event) {
            Ok(flow) => *control_flow = flow,
            Err(error) => {
                error!("{:?}", error);
                *control_flow = ControlFlow::Exit;
            }
        });
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        let _guard = self.runtime.enter();
        if let Err(error) = self.runtime.block_on(self.application.shutdown()) {
            error!("{:?}", error);
        }
    }
}
