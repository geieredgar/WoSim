mod create_world;
mod delete_world;
mod installation_option;
mod root;
mod server;
mod world;

pub use create_world::*;
pub use delete_world::*;
use installation_option::*;
pub use root::*;
pub use server::*;
pub use world::*;

use iced::{Align, Container, Element, Length, Row, Space, Text};

use crate::{
    icon::Icon,
    message::Message,
    style::{BackgroundContainer, ForegroundContainer},
    theme::Theme,
};

pub fn page<'a>(content: impl Into<Element<'a, Message>>, theme: Theme) -> Container<'a, Message> {
    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .style(BackgroundContainer(theme))
}

pub fn header<'a>(
    content: impl Into<Element<'a, Message>>,
    theme: Theme,
) -> Container<'a, Message> {
    Container::new(
        Row::new()
            .push(Icon::Joystick.svg(theme.colors().primary.bright, 32, 32))
            .push(
                Text::new("WoSim")
                    .size(30)
                    .color(theme.colors().primary.bright),
            )
            .push(Space::with_width(Length::Units(10)))
            .push(content)
            .align_items(Align::Center)
            .width(Length::Fill),
    )
    .padding(10)
    .style(ForegroundContainer(theme, 0.0))
}

pub fn dialog<'a>(
    content: impl Into<Element<'a, Message>>,
    theme: Theme,
) -> Container<'a, Message> {
    Container::new(
        Container::new(content)
            .padding(20)
            .style(ForegroundContainer(theme, 20.0)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(20)
    .center_x()
    .center_y()
}
