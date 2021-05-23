use iced::{
    button, scrollable, text_input, Align, Button, Column, Container, Length, Scrollable, Text,
    TextInput,
};

use crate::{
    installation::Installation,
    message::Message,
    style::{DefaultTextInput, PrimaryButton},
    theme::Theme,
};

use super::{dialog, InstallationOptionComponent};

pub struct CreateWorldComponent {
    name: String,
    version_options: Vec<InstallationOptionComponent>,
    version: Option<Installation>,
    state: CreateWorldState,
}

#[derive(Default)]
pub struct CreateWorldState {
    create: button::State,
    name: text_input::State,
    version_options: scrollable::State,
}

impl CreateWorldComponent {
    pub(super) fn new(versions: &[Installation]) -> Self {
        let version_options = versions
            .iter()
            .cloned()
            .map(InstallationOptionComponent::new)
            .collect();
        Self {
            name: "".to_owned(),
            version: versions.first().cloned(),
            version_options,
            state: CreateWorldState::default(),
        }
    }

    pub(super) fn update(&mut self, message: Message) {
        match message {
            Message::ChangeName(name) => self.name = name,
            Message::SelectVersion(version) => self.version = Some(version),
            _ => panic!(),
        }
    }

    pub(super) fn view(&mut self, theme: Theme) -> Container<'_, Message> {
        let version = self.version.clone();
        dialog(
            Column::new()
                .push(
                    Text::new("World creation")
                        .size(40)
                        .color(theme.colors().primary.bright),
                )
                .push(Text::new("Name:"))
                .push(
                    TextInput::new(
                        &mut self.state.name,
                        "New World",
                        &self.name,
                        Message::ChangeName,
                    )
                    .padding(5)
                    .style(DefaultTextInput(theme)),
                )
                .push(Text::new("Version:"))
                .push(
                    Scrollable::new(&mut self.state.version_options).push(Column::with_children(
                        self.version_options
                            .iter_mut()
                            .map(|option| option.view(theme, version.as_ref()).into())
                            .collect(),
                    )),
                )
                .push(
                    Container::new({
                        let mut button =
                            Button::new(&mut self.state.create, Text::new("Create world"))
                                .padding(5)
                                .style(PrimaryButton(theme));
                        if let Some(version) = self.version.clone() {
                            button =
                                button.on_press(Message::CreateWorld(self.name.clone(), version))
                        }
                        button
                    })
                    .width(Length::Fill)
                    .align_x(Align::End),
                )
                .width(Length::Fill)
                .spacing(20)
                .padding(20),
            theme,
        )
    }
}
