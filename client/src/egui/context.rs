use std::sync::Arc;

use egui::{ClippedMesh, CtxRef, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Vec2};

use copypasta::{ClipboardContext, ClipboardProvider};

use winit::{
    event::{
        ElementState, ModifiersState, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent,
    },
    window::{CursorIcon, Window},
};

use util::{handle::HandleFlow, shader::align_bytes};
use vulkan::{
    DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateFlags,
    DescriptorType, Device, Filter, PipelineLayout, PipelineLayoutCreateFlags, PushConstantRange,
    Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode, ShaderModule,
    ShaderModuleCreateFlags, ShaderStageFlags, LOD_CLAMP_NONE,
};

use crate::{
    error::Error,
    shaders::{EGUI_FRAG, EGUI_VERT},
    ApplicationMessage,
};

use super::Font;

pub struct EguiContext {
    pub(super) inner: CtxRef,
    input: RawInput,
    pub(super) pipeline_layout: PipelineLayout,
    pub(super) vertex_shader_module: ShaderModule,
    pub(super) fragment_shader_module: ShaderModule,
    pub(super) sampler: Sampler,
    pub(super) meshes: Arc<Vec<ClippedMesh>>,
    pub(super) set_layout: DescriptorSetLayout,
    pub(super) font: Option<Arc<Font>>,
    cursor_pos: Pos2,
    modifiers: Modifiers,
    clipboard: ClipboardContext,
}

impl EguiContext {
    pub fn new(device: &Arc<Device>, scale_factor: f32) -> Result<Self, Error> {
        let inner = CtxRef::default();
        let mut input = RawInput {
            pixels_per_point: Some(scale_factor),
            ..Default::default()
        };
        input.pixels_per_point = Some(scale_factor);
        let vertex_shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(EGUI_VERT.load()?.bytes()),
        )?;
        let fragment_shader_module = device.create_shader_module(
            ShaderModuleCreateFlags::empty(),
            &align_bytes(EGUI_FRAG.load()?.bytes()),
        )?;
        let bindings = [DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(ShaderStageFlags::FRAGMENT)
            .build()];
        let set_layout = device
            .create_descriptor_set_layout(DescriptorSetLayoutCreateFlags::empty(), &bindings)?;
        let set_layouts = [&set_layout];
        let push_constant_ranges = [
            PushConstantRange::builder()
                .offset(0)
                .size(8)
                .stage_flags(ShaderStageFlags::VERTEX)
                .build(),
            PushConstantRange::builder()
                .offset(8)
                .size(4)
                .stage_flags(ShaderStageFlags::FRAGMENT)
                .build(),
        ];
        let pipeline_layout = device.create_pipeline_layout(
            PipelineLayoutCreateFlags::empty(),
            &set_layouts,
            &push_constant_ranges,
        )?;
        let sampler = device.create_sampler(
            &SamplerCreateInfo::builder()
                .address_mode_u(SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_v(SamplerAddressMode::CLAMP_TO_EDGE)
                .address_mode_w(SamplerAddressMode::CLAMP_TO_EDGE)
                .anisotropy_enable(false)
                .min_filter(Filter::LINEAR)
                .mag_filter(Filter::LINEAR)
                .mipmap_mode(SamplerMipmapMode::LINEAR)
                .min_lod(0.0)
                .max_lod(LOD_CLAMP_NONE),
        )?;
        Ok(Self {
            inner,
            input,
            meshes: Arc::new(Vec::new()),
            vertex_shader_module,
            fragment_shader_module,
            pipeline_layout,
            set_layout,
            sampler,
            font: None,
            cursor_pos: Pos2::default(),
            modifiers: Modifiers::default(),
            clipboard: ClipboardContext::new().map_err(super::Error::NewClipboardFailed)?,
        })
    }

    pub fn handle_event(&mut self, message: &ApplicationMessage) -> HandleFlow {
        if let ApplicationMessage::WindowEvent(event) = message {
            match event {
                WindowEvent::Resized(size) => {
                    let pixels_per_point = self
                        .input
                        .pixels_per_point
                        .unwrap_or_else(|| self.inner.pixels_per_point());
                    self.input.screen_rect = Some(egui::Rect::from_min_size(
                        Default::default(),
                        Vec2 {
                            x: size.width as f32,
                            y: size.height as f32,
                        } / pixels_per_point,
                    ));
                }
                WindowEvent::ReceivedCharacter(c) => {
                    if self.inner.wants_keyboard_input() && !c.is_ascii_control() {
                        self.input.events.push(Event::Text(c.to_string()));
                        return HandleFlow::Handled;
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    if self.inner.wants_keyboard_input() {
                        if let Some(code) = input.virtual_keycode {
                            if let Some(key) = key(code) {
                                self.input.events.push(Event::Key {
                                    key,
                                    pressed: input.state == ElementState::Pressed,
                                    modifiers: self.modifiers,
                                });
                                return HandleFlow::Handled;
                            }
                        }
                    }
                }
                WindowEvent::ModifiersChanged(state) => {
                    self.modifiers = modifiers(*state);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let scale_factor = self.scale_factor();
                    self.cursor_pos = Pos2 {
                        x: position.x as f32 / scale_factor,
                        y: position.y as f32 / scale_factor,
                    };
                    self.input
                        .events
                        .push(egui::Event::PointerMoved(self.cursor_pos));
                }
                WindowEvent::CursorLeft { .. } => {
                    self.input.events.push(Event::PointerGone);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    match *delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            self.input.scroll_delta = Vec2 { x, y } * 24.0;
                        }
                        MouseScrollDelta::PixelDelta(delta) => {
                            self.input.scroll_delta = Vec2 {
                                x: delta.x as f32,
                                y: delta.y as f32,
                            };
                        }
                    };
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if let Some(button) = pointer_button(*button) {
                        self.input.events.push(Event::PointerButton {
                            pos: self.cursor_pos,
                            button,
                            pressed: *state == ElementState::Pressed,
                            modifiers: self.modifiers,
                        });
                        if self.inner.wants_pointer_input() {
                            return HandleFlow::Handled;
                        }
                    }
                }
                _ => {}
            }
        }
        if let ApplicationMessage::ScaleFactorChanged(scale_factor, new_inner_size) = message {
            self.input.pixels_per_point = Some(*scale_factor as f32);
            self.input.screen_rect = Some(egui::Rect::from_min_size(
                Default::default(),
                Vec2 {
                    x: new_inner_size.width as f32,
                    y: new_inner_size.height as f32,
                } / self.scale_factor(),
            ));
        }
        HandleFlow::Unhandled
    }

    pub fn begin(&mut self) -> CtxRef {
        self.inner.begin_frame(self.input.take());
        self.inner.clone()
    }

    pub fn end(&mut self, window: Option<&Window>) -> Result<(), Error> {
        let (output, shapes) = self.inner.end_frame();
        let meshes = Arc::new(self.inner.tessellate(shapes));
        self.meshes = meshes;
        if let Some(window) = window {
            if let Some(icon) = cursor(output.cursor_icon) {
                window.set_cursor_icon(icon);
                window.set_cursor_visible(true);
            } else {
                window.set_cursor_visible(false);
            }
        }
        if !output.copied_text.is_empty() {
            self.clipboard
                .set_contents(output.copied_text)
                .map_err(super::Error::SetClipboardContents)?;
        }
        if let Some(open_url) = output.open_url {
            webbrowser::open(&open_url.url)?;
        }
        Ok(())
    }

    fn scale_factor(&self) -> f32 {
        self.input
            .pixels_per_point
            .unwrap_or_else(|| self.inner.pixels_per_point())
    }
}

fn key(code: VirtualKeyCode) -> Option<Key> {
    Some(match code {
        VirtualKeyCode::Key1 => Key::Num1,
        VirtualKeyCode::Key2 => Key::Num2,
        VirtualKeyCode::Key3 => Key::Num3,
        VirtualKeyCode::Key4 => Key::Num4,
        VirtualKeyCode::Key5 => Key::Num5,
        VirtualKeyCode::Key6 => Key::Num6,
        VirtualKeyCode::Key7 => Key::Num7,
        VirtualKeyCode::Key8 => Key::Num8,
        VirtualKeyCode::Key9 => Key::Num9,
        VirtualKeyCode::Key0 => Key::Num0,
        VirtualKeyCode::A => Key::A,
        VirtualKeyCode::B => Key::B,
        VirtualKeyCode::C => Key::C,
        VirtualKeyCode::D => Key::D,
        VirtualKeyCode::E => Key::E,
        VirtualKeyCode::F => Key::F,
        VirtualKeyCode::G => Key::G,
        VirtualKeyCode::H => Key::H,
        VirtualKeyCode::I => Key::I,
        VirtualKeyCode::J => Key::J,
        VirtualKeyCode::K => Key::K,
        VirtualKeyCode::L => Key::L,
        VirtualKeyCode::M => Key::M,
        VirtualKeyCode::N => Key::N,
        VirtualKeyCode::O => Key::O,
        VirtualKeyCode::P => Key::P,
        VirtualKeyCode::Q => Key::Q,
        VirtualKeyCode::R => Key::R,
        VirtualKeyCode::S => Key::S,
        VirtualKeyCode::T => Key::T,
        VirtualKeyCode::U => Key::U,
        VirtualKeyCode::V => Key::V,
        VirtualKeyCode::W => Key::W,
        VirtualKeyCode::X => Key::X,
        VirtualKeyCode::Y => Key::Y,
        VirtualKeyCode::Z => Key::Z,
        VirtualKeyCode::Escape => Key::Escape,
        VirtualKeyCode::Insert => Key::Insert,
        VirtualKeyCode::Home => Key::Home,
        VirtualKeyCode::Delete => Key::Delete,
        VirtualKeyCode::End => Key::End,
        VirtualKeyCode::PageDown => Key::PageDown,
        VirtualKeyCode::PageUp => Key::PageUp,
        VirtualKeyCode::Left => Key::ArrowLeft,
        VirtualKeyCode::Up => Key::ArrowUp,
        VirtualKeyCode::Right => Key::ArrowRight,
        VirtualKeyCode::Down => Key::ArrowDown,
        VirtualKeyCode::Back => Key::Backspace,
        VirtualKeyCode::Return => Key::Enter,
        VirtualKeyCode::Space => Key::Space,
        VirtualKeyCode::Numpad0 => Key::Num0,
        VirtualKeyCode::Numpad1 => Key::Num1,
        VirtualKeyCode::Numpad2 => Key::Num2,
        VirtualKeyCode::Numpad3 => Key::Num3,
        VirtualKeyCode::Numpad4 => Key::Num4,
        VirtualKeyCode::Numpad5 => Key::Num5,
        VirtualKeyCode::Numpad6 => Key::Num6,
        VirtualKeyCode::Numpad7 => Key::Num7,
        VirtualKeyCode::Numpad8 => Key::Num8,
        VirtualKeyCode::Numpad9 => Key::Num9,
        VirtualKeyCode::NumpadEnter => Key::Enter,
        VirtualKeyCode::Tab => Key::Tab,
        _ => return None,
    })
}

fn modifiers(state: ModifiersState) -> Modifiers {
    Modifiers {
        alt: state.alt(),
        ctrl: state.ctrl(),
        #[cfg(target_os = "macos")]
        mac_cmd: state.logo(),
        #[cfg(not(target_os = "macos"))]
        mac_cmd: false,
        #[cfg(target_os = "macos")]
        command: state.logo(),
        #[cfg(not(target_os = "macos"))]
        command: state.ctrl(),
        shift: state.shift(),
    }
}

fn pointer_button(button: MouseButton) -> Option<PointerButton> {
    Some(match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        _ => return None,
    })
}

fn cursor(icon: egui::CursorIcon) -> Option<CursorIcon> {
    Some(match icon {
        egui::CursorIcon::Default => CursorIcon::Default,
        egui::CursorIcon::None => return None,
        egui::CursorIcon::ContextMenu => CursorIcon::ContextMenu,
        egui::CursorIcon::Help => CursorIcon::Help,
        egui::CursorIcon::PointingHand => CursorIcon::Hand,
        egui::CursorIcon::Progress => CursorIcon::Progress,
        egui::CursorIcon::Wait => CursorIcon::Wait,
        egui::CursorIcon::Cell => CursorIcon::Cell,
        egui::CursorIcon::Crosshair => CursorIcon::Crosshair,
        egui::CursorIcon::Text => CursorIcon::Text,
        egui::CursorIcon::VerticalText => CursorIcon::VerticalText,
        egui::CursorIcon::Alias => CursorIcon::Alias,
        egui::CursorIcon::Copy => CursorIcon::Copy,
        egui::CursorIcon::Move => CursorIcon::Move,
        egui::CursorIcon::NoDrop => CursorIcon::NoDrop,
        egui::CursorIcon::NotAllowed => CursorIcon::NotAllowed,
        egui::CursorIcon::Grab => CursorIcon::Grab,
        egui::CursorIcon::Grabbing => CursorIcon::Grabbing,
        egui::CursorIcon::AllScroll => CursorIcon::AllScroll,
        egui::CursorIcon::ResizeHorizontal => CursorIcon::ColResize,
        egui::CursorIcon::ResizeNeSw => CursorIcon::NeswResize,
        egui::CursorIcon::ResizeNwSe => CursorIcon::NwseResize,
        egui::CursorIcon::ResizeVertical => CursorIcon::RowResize,
        egui::CursorIcon::ZoomIn => CursorIcon::ZoomIn,
        egui::CursorIcon::ZoomOut => CursorIcon::ZoomOut,
    })
}
