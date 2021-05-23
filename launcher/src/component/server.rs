use iced::{button, Align, Button, Column, Container, Length, Row, Space, Text};

use crate::{
    icon::Icon,
    message::Message,
    server::Server,
    style::{ButtonContainer, ForegroundContainer, InlineButton},
    theme::Theme,
};

pub struct ServerComponent {
    server: Server,
    state: ServerState,
}

#[derive(Default)]
struct ServerState {
    join: button::State,
}

impl ServerComponent {
    pub fn new(server: Server) -> Self {
        Self {
            server,
            state: ServerState::default(),
        }
    }

    pub fn view(&mut self, theme: Theme) -> Container<'_, Message> {
        Container::new(
            Container::new(
                Row::new()
                    .width(Length::Fill)
                    .spacing(10)
                    .align_items(Align::Center)
                    .push(
                        Column::new()
                            .push(Text::new(format!(
                                "{} [{}]",
                                self.server.name, self.server.address
                            )))
                            .push(Text::new(self.server.description.to_string())),
                    )
                    .push(Space::with_width(Length::Fill))
                    .push(
                        Container::new(
                            Button::new(
                                &mut self.state.join,
                                Icon::Play.svg(theme.colors().primary.bright, 32, 32),
                            )
                            .style(InlineButton(theme))
                            .on_press(Message::JoinServer(self.server.clone())),
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
