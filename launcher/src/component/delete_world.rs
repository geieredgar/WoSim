use iced::{button, Align, Button, Column, Container, Row, Text};

use crate::{
    component::dialog, message::Message, style::PrimaryButton, theme::Theme, world::World,
};

pub struct DeleteWorldComponent {
    world: World,
    state: DeleteWorldState,
}

#[derive(Default)]
pub struct DeleteWorldState {
    confirm: button::State,
    cancel: button::State,
}

impl DeleteWorldComponent {
    pub(super) fn new(world: World) -> Self {
        Self {
            world,
            state: DeleteWorldState::default(),
        }
    }

    pub(super) fn view(&mut self, theme: Theme) -> Container<'_, Message> {
        dialog(Column::new().push(
                Text::new(format!(
                        "Do you really want to delete '{}'?",
                        self.world.path.file_name().unwrap().to_string_lossy()
                    ))
                    .size(40)
                    .color(theme.colors().primary.bright)
            )
            .push(Text::new(format!(
                "This will delete the directory '{}' and all its content. This operation is irreversible.",
                self.world.path.display()
            )))
            .push(
                Row::new()
                    .push(
                        Button::new(&mut self.state.cancel, Text::new("No"))
                            .padding(5)
                            .style(PrimaryButton(theme))
                            .on_press(Message::SelectWorldTab),
                    )
                    .push(
                        Button::new(&mut self.state.confirm, Text::new("Yes"))
                            .padding(5)
                            .style(PrimaryButton(theme))
                            .on_press(Message::DeleteWorld(self.world.clone())),
                    )
                    .spacing(100),
            ).spacing(20).align_items(Align::Center), theme)
    }
}
