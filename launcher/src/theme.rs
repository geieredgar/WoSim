use std::fmt::Display;

use iced::{Background, Color};

#[derive(Clone, Copy)]
pub enum Theme {
    Dark,
}

pub struct ColorPalette {
    pub primary: ColorVariants,
    pub background: RGB,
    pub foreground: RGB,
    pub surface: ColorVariants,
}

pub struct ColorVariants {
    pub bright: RGB,
    pub normal: RGB,
}

#[derive(Clone, Copy)]
pub struct RGB(u32);

impl From<RGB> for Color {
    fn from(rgb: RGB) -> Self {
        let bytes = rgb.0.to_be_bytes();
        Self::from_rgb8(bytes[1], bytes[2], bytes[3])
    }
}

impl From<RGB> for Background {
    fn from(rgb: RGB) -> Self {
        let color: Color = rgb.into();
        color.into()
    }
}

static DARK_PALETTE: ColorPalette = ColorPalette {
    primary: ColorVariants {
        normal: RGB(0x035411),
        bright: RGB(0x0dd166),
    },
    background: RGB(0x131313),
    foreground: RGB(0x262626),
    surface: ColorVariants {
        bright: RGB(0xffffff),
        normal: RGB(0x5f5f5f),
    },
};

impl Display for RGB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:06x}", self.0)
    }
}

impl Theme {
    pub fn colors(self) -> &'static ColorPalette {
        match self {
            Theme::Dark => &DARK_PALETTE,
        }
    }
}
