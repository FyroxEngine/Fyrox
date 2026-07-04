use crate::fyrox::{
    core::color::Color,
    gui::{brush::Brush, style::StyledProperty},
};

pub const BG: Color = Color::opaque(0x0b, 0x0f, 0x19);
pub const PANEL_DARK: Color = Color::opaque(0x11, 0x15, 0x1f);
pub const PANEL_LIGHT: Color = Color::opaque(0x1a, 0x1f, 0x2b);
pub const PANEL_BORDER: Color = Color::opaque(0x33, 0x3c, 0x4f);
pub const PRIMARY: Color = Color::opaque(0xff, 0xd7, 0x00);
pub const SECONDARY: Color = Color::opaque(0x3b, 0x82, 0xf6);
pub const ACCENT: Color = Color::opaque(0xa8, 0x55, 0xf7);
pub const PINK: Color = Color::opaque(0xec, 0x48, 0x99);
pub const TEXT_GRAY: Color = Color::opaque(0xd1, 0xd5, 0xdb);

pub fn brush(c: Color) -> StyledProperty<Brush> {
    Brush::Solid(c).into()
}

pub fn with_alpha(c: Color, a: u8) -> Color {
    Color::from_rgba(c.r, c.g, c.b, a)
}
