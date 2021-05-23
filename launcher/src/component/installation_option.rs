use iced::{button, Align, Button, Container, Length, Row, Text};

use crate::{installation::Installation, message::Message, style::ForegroundOption, theme::Theme};

pub(super) struct InstallationOptionComponent {
    installation: Installation,
    state: InstallationOptionState,
}

#[derive(Default)]
struct InstallationOptionState {
    click: button::State,
}

impl InstallationOptionComponent {
    pub(super) fn new(version: Installation) -> Self {
        Self {
            installation: version,
            state: Default::default(),
        }
    }

    pub(super) fn view(
        &mut self,
        theme: Theme,
        selected: Option<&Installation>,
    ) -> Container<'_, Message> {
        Container::new(
            Button::new(
                &mut self.state.click,
                Row::new()
                    .width(Length::Fill)
                    .align_items(Align::Center)
                    .push(Text::new(self.installation.info.name.clone())),
            )
            .width(Length::Fill)
            .padding(10)
            .style(ForegroundOption(
                theme,
                Some(&self.installation.path) == selected.map(|version| &version.path),
            ))
            .on_press(Message::SelectVersion(self.installation.clone())),
        )
        .width(Length::Fill)
        .padding(10)
    }
}
