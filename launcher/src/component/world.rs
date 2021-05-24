use iced::{button, Align, Button, Container, Length, Row, Space, Text};

use crate::{
    icon::Icon,
    message::Message,
    style::{ButtonContainer, ForegroundContainer, InlineButton},
    theme::Theme,
    world::World,
};

pub struct WorldComponent {
    pub world: World,
    state: WorldState,
}

#[derive(Default)]
struct WorldState {
    play: button::State,
    delete: button::State,
}

impl WorldComponent {
    pub(super) fn new(world: World) -> Self {
        Self {
            world,
            state: WorldState::default(),
        }
    }

    pub(super) fn view(&mut self, theme: Theme) -> Container<'_, Message> {
        Container::new(
            Container::new(
                Row::new()
                    .width(Length::Fill)
                    .align_items(Align::Center)
                    .push(Text::new(format!(
                        "{}",
                        self.world.path.file_name().unwrap().to_string_lossy()
                    )))
                    .push(Space::with_width(Length::Fill))
                    .push(
                        Container::new(
                            Row::new()
                                .push(
                                    Button::new(
                                        &mut self.state.delete,
                                        Icon::Trash.svg(theme.colors().primary.bright, 32, 32),
                                    )
                                    .style(InlineButton(theme))
                                    .on_press(Message::SetupDeleteWorld(self.world.clone())),
                                )
                                .push(
                                    Button::new(
                                        &mut self.state.play,
                                        Icon::Play.svg(theme.colors().primary.bright, 32, 32),
                                    )
                                    .style(InlineButton(theme))
                                    .on_press(Message::PlayWorld(self.world.clone())),
                                ),
                        )
                        .padding(3)
                        .style(ButtonContainer(theme)),
                    ),
            )
            .padding(10)
            .style(ForegroundContainer(theme, 10.0)),
        )
        .padding(10)
    }
}
