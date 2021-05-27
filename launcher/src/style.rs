use iced::{button, container, text_input, Color};

use crate::{
    icon,
    theme::{Theme, RGB},
};

pub struct PrimaryButton(pub Theme);
pub struct DefaultTextInput(pub Theme);
pub struct SelectableButton(pub Theme, pub bool);
pub struct ForegroundOption(pub Theme, pub bool);
pub struct BackgroundContainer(pub Theme);
pub struct ForegroundContainer(pub Theme, pub f32);
pub struct LogoContainer(pub Theme);
pub struct ButtonContainer(pub Theme);
pub struct InlineButton(pub Theme);

impl text_input::StyleSheet for DefaultTextInput {
    fn active(&self) -> text_input::Style {
        text_input::Style {
            background: self.0.colors().background.into(),
            ..Default::default()
        }
    }

    fn focused(&self) -> text_input::Style {
        text_input::Style {
            background: self.0.colors().background.into(),
            border_color: self.0.colors().surface.bright.into(),
            border_width: 1.0,
            ..Default::default()
        }
    }

    fn placeholder_color(&self) -> Color {
        self.0.colors().surface.normal.into()
    }

    fn value_color(&self) -> Color {
        self.0.colors().surface.bright.into()
    }

    fn selection_color(&self) -> Color {
        self.0.colors().primary.normal.into()
    }
}

impl button::StyleSheet for SelectableButton {
    fn active(&self) -> button::Style {
        button::Style {
            background: None,
            text_color: if self.1 {
                self.0.colors().surface.bright.into()
            } else {
                self.0.colors().surface.normal.into()
            },
            ..Default::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: None,
            text_color: if self.1 {
                self.0.colors().surface.bright.into()
            } else {
                self.0.colors().surface.normal.into()
            },
            ..Default::default()
        }
    }
}

impl icon::StyleSheet for SelectableButton {
    fn color(&self) -> RGB {
        if self.1 {
            self.0.colors().surface.bright
        } else {
            self.0.colors().surface.normal
        }
    }
}

impl button::StyleSheet for ForegroundOption {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(self.0.colors().foreground.into()),
            text_color: self.0.colors().surface.bright.into(),
            border_color: self.0.colors().primary.bright.into(),
            border_width: if self.1 { 1.0 } else { 0.0 },
            ..Default::default()
        }
    }
}

impl button::StyleSheet for PrimaryButton {
    fn active(&self) -> button::Style {
        button::Style {
            background: None,
            text_color: self.0.colors().primary.bright.into(),
            border_color: self.0.colors().primary.normal.into(),
            border_width: 1.0,
            border_radius: 3.0,
            ..Default::default()
        }
    }

    fn hovered(&self) -> button::Style {
        match self.0 {
            Theme::Dark => button::Style {
                background: Some(self.0.colors().primary.normal.into()),
                text_color: self.0.colors().primary.bright.into(),
                border_color: self.0.colors().primary.normal.into(),
                border_width: 1.0,
                border_radius: 3.0,
                ..Default::default()
            },
        }
    }
}

impl icon::StyleSheet for PrimaryButton {
    fn color(&self) -> RGB {
        self.0.colors().primary.bright
    }
}

impl button::StyleSheet for InlineButton {
    fn active(&self) -> button::Style {
        button::Style {
            background: None,
            text_color: self.0.colors().primary.bright.into(),
            border_radius: 3.0,
            ..Default::default()
        }
    }

    fn hovered(&self) -> button::Style {
        match self.0 {
            Theme::Dark => button::Style {
                background: Some(self.0.colors().primary.normal.into()),
                text_color: self.0.colors().primary.bright.into(),
                border_radius: 3.0,
                ..Default::default()
            },
        }
    }
}

impl icon::StyleSheet for InlineButton {
    fn color(&self) -> RGB {
        self.0.colors().primary.bright
    }
}

impl container::StyleSheet for ButtonContainer {
    fn style(&self) -> container::Style {
        container::Style {
            background: None,
            border_color: self.0.colors().primary.normal.into(),
            border_radius: 5.0,
            border_width: 1.0,
            ..Default::default()
        }
    }
}

impl container::StyleSheet for BackgroundContainer {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(self.0.colors().background.into()),
            text_color: Some(self.0.colors().surface.normal.into()),
            ..Default::default()
        }
    }
}

impl container::StyleSheet for ForegroundContainer {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(self.0.colors().foreground.into()),
            text_color: Some(self.0.colors().surface.bright.into()),
            border_radius: self.1,
            ..Default::default()
        }
    }
}

impl container::StyleSheet for LogoContainer {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(self.0.colors().primary.bright.into()),
            text_color: Some(self.0.colors().foreground.into()),
            border_radius: 5.0,
            ..Default::default()
        }
    }
}
