use iced::{svg::Handle, Length, Svg};

use crate::theme::RGB;

#[derive(Clone, Copy)]
pub enum Icon {
    Play,
    Trash,
}

impl Icon {
    pub fn handle(self, color: RGB) -> Handle {
        let input = match self {
            Icon::Play => format!(
                r###"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="{}" class="bi bi-play" viewBox="0 0 16 16">
                    <path d="M10.804 8 5 4.633v6.734L10.804 8zm.792-.696a.802.802 0 0 1 0 1.392l-6.363 3.692C4.713 12.69 4 12.345 4 11.692V4.308c0-.653.713-.998 1.233-.696l6.363 3.692z"/>
                </svg>"###,
                color
            ),
            Icon::Trash => format!(
                r###"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="{}" class="bi bi-trash" viewBox="0 0 16 16">
                    <path d="M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0V6z"/>
                    <path fill-rule="evenodd" d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1v1zM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z"/>
                </svg>"###,
                color
            ),
        };
        Handle::from_memory(input)
    }

    pub fn svg(self, width: u16, height: u16, style: impl StyleSheet) -> Svg {
        Svg::new(self.handle(style.color()))
            .width(Length::Units(width))
            .height(Length::Units(height))
    }
}

pub trait StyleSheet {
    fn color(&self) -> RGB;
}
