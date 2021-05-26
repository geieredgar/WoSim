mod component;
mod icon;
mod installation;
mod message;
mod scan;
mod server;
mod shell;
mod style;
mod theme;
mod world;

use iced::{executor, Command, Settings, Subscription};
use iced_winit::{Application, Clipboard, Program};
use log::error;
use message::Message;

use crate::component::RootComponent;

struct Launcher(RootComponent);

impl Program for Launcher {
    type Renderer = iced_wgpu::Renderer;

    type Message = Message;

    type Clipboard = Clipboard;

    fn update(
        &mut self,
        message: Self::Message,
        _clipboard: &mut Self::Clipboard,
    ) -> Command<Self::Message> {
        match self.0.update(message) {
            Ok(command) => command,
            Err(error) => {
                error!("{}", error);
                Command::none()
            }
        }
    }

    fn view(&mut self) -> iced_native::Element<'_, Self::Message, Self::Renderer> {
        self.0.view().into()
    }
}

impl Application for Launcher {
    type Flags = ();

    fn new(_: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        (Self(RootComponent::default()), Command::none())
    }

    fn title(&self) -> String {
        format!(
            "WoSim Launcher v{}.{}.{}",
            env!("CARGO_PKG_VERSION_MAJOR").parse::<String>().unwrap(),
            env!("CARGO_PKG_VERSION_MINOR").parse::<String>().unwrap(),
            env!("CARGO_PKG_VERSION_PATCH").parse::<String>().unwrap()
        )
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        self.0.subscription()
    }
}

impl shell::Application for Launcher {
    fn is_visible(&self) -> bool {
        self.0.is_visible()
    }
}

fn main() -> iced::Result {
    env_logger::init();
    let settings = Settings::with_flags(());
    let renderer_settings = iced_wgpu::Settings {
        default_font: settings.default_font,
        default_text_size: settings.default_text_size,
        antialiasing: if settings.antialiasing {
            Some(iced_wgpu::settings::Antialiasing::MSAAx4)
        } else {
            None
        },
        ..iced_wgpu::Settings::from_env()
    };
    Ok(shell::run::<
        Launcher,
        executor::Default,
        iced_wgpu::window::Compositor,
    >(settings.into(), renderer_settings)?)
}
