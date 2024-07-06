use crate::{
    algebra::{Vector3, Vector4},
    reflect::prelude::*,
    uuid_provider,
    visitor::{Visit, VisitResult, Visitor},
};
use bytemuck::{Pod, Zeroable};
use num_traits::Zero;
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Eq, Visit, Reflect, Pod, Zeroable)]
#[repr(C)]
pub struct Color {
    // Do not change order! OpenGL requires this order!
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

uuid_provider!(Color = "74e898aa-de19-44bd-8213-3b6d450b1bf8");

impl Default for Color {
    #[inline]
    fn default() -> Self {
        Self::WHITE
    }
}

impl Into<u32> for Color {
    #[inline]
    fn into(self) -> u32 {
        ((self.a as u32) << 24) | ((self.b as u32) << 16) | ((self.g as u32) << 8) | (self.r as u32)
    }
}

impl From<Vector3<f32>> for Color {
    fn from(v: Vector3<f32>) -> Self {
        Self {
            r: (v.x.clamp(0.0, 1.0) * 255.0) as u8,
            g: (v.y.clamp(0.0, 1.0) * 255.0) as u8,
            b: (v.z.clamp(0.0, 1.0) * 255.0) as u8,
            a: 255,
        }
    }
}

impl From<Vector4<f32>> for Color {
    fn from(v: Vector4<f32>) -> Self {
        Self {
            r: (v.x.clamp(0.0, 1.0) * 255.0) as u8,
            g: (v.y.clamp(0.0, 1.0) * 255.0) as u8,
            b: (v.z.clamp(0.0, 1.0) * 255.0) as u8,
            a: (v.w.clamp(0.0, 1.0) * 255.0) as u8,
        }
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Visit, Reflect)]
pub struct Hsv {
    /// [0; 360] range
    hue: f32,
    /// [0; 100] range
    saturation: f32,
    /// [0; 100] range
    brightness: f32,
}

impl Hsv {
    #[inline]
    pub fn new(hue: f32, saturation: f32, brightness: f32) -> Self {
        Self {
            hue: hue.clamp(0.0, 360.0),
            saturation: saturation.clamp(0.0, 100.0),
            brightness: brightness.clamp(0.0, 100.0),
        }
    }

    #[inline]
    pub fn hue(&self) -> f32 {
        self.hue
    }

    #[inline]
    pub fn set_hue(&mut self, hue: f32) {
        self.hue = hue.clamp(0.0, 360.0);
    }

    #[inline]
    pub fn saturation(&self) -> f32 {
        self.saturation
    }

    #[inline]
    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation.clamp(0.0, 100.0);
    }

    #[inline]
    pub fn brightness(&self) -> f32 {
        self.brightness
    }

    #[inline]
    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness = brightness.clamp(0.0, 100.0);
    }
}

impl From<Color> for Hsv {
    #[inline]
    fn from(color: Color) -> Self {
        let r = color.r as f32 / 255.0;
        let g = color.g as f32 / 255.0;
        let b = color.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);

        let hue = if max.eq(&min) {
            0.0 // Undefined.
        } else if max.eq(&r) && g >= b {
            60.0 * (g - b) / (max - min)
        } else if max.eq(&r) && g < b {
            60.0 * (g - b) / (max - min) + 360.0
        } else if max.eq(&g) {
            60.0 * (b - r) / (max - min) + 120.0
        } else if max.eq(&b) {
            60.0 * (r - g) / (max - min) + 240.0
        } else {
            0.0 // Undefined.
        };

        let saturation = if max.eq(&0.0) { 0.0 } else { 1.0 - min / max };

        let brightness = max;

        Self {
            hue,
            saturation: saturation * 100.0,
            brightness: brightness * 100.0,
        }
    }
}

impl From<Hsv> for Color {
    #[inline]
    fn from(hsv: Hsv) -> Self {
        let hi = ((hsv.hue / 60.0) % 6.0) as i32;
        let vmin = ((100.0 - hsv.saturation) * hsv.brightness) / 100.0;
        let a = (hsv.brightness - vmin) * ((hsv.hue % 60.0) / 60.0);
        let vinc = vmin + a;
        let vdec = hsv.brightness - a;
        Self::from(
            match hi {
                0 => Vector3::new(hsv.brightness, vinc, vmin),
                1 => Vector3::new(vdec, hsv.brightness, vmin),
                2 => Vector3::new(vmin, hsv.brightness, vinc),
                3 => Vector3::new(vmin, vdec, hsv.brightness),
                4 => Vector3::new(vinc, vmin, hsv.brightness),
                5 => Vector3::new(hsv.brightness, vmin, vdec),
                _ => unreachable!(),
            }
            .scale(1.0 / 100.0),
        )
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Hsl {
    /// [0; 360] range
    hue: f32,
    /// [0; 1] range
    saturation: f32,
    /// [0; 1] range
    lightness: f32,
}

impl Hsl {
    /// Hue: [0; 360] range
    /// Saturation: [0; 1] range
    /// Lightness: [0; 1] range
    pub fn new(hue: f32, saturation: f32, lightness: f32) -> Self {
        Self {
            hue: hue.abs() % 360.0,
            saturation: saturation.clamp(0.0, 1.0),
            lightness: lightness.clamp(0.0, 1.0),
        }
    }

    pub fn hue(&self) -> f32 {
        self.hue
    }

    pub fn set_hue(&mut self, hue: f32) {
        self.hue = hue.abs() % 360.0;
    }

    pub fn saturation(&self) -> f32 {
        self.saturation
    }

    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation.clamp(0.0, 1.0)
    }

    pub fn lightness(&self) -> f32 {
        self.lightness
    }

    pub fn set_lightness(&mut self, lightness: f32) {
        self.lightness = lightness.clamp(0.0, 1.0)
    }
}

impl From<Hsl> for Color {
    #[allow(clippy::manual_range_contains)]
    #[inline]
    fn from(v: Hsl) -> Self {
        let h = v.hue;
        let s = v.saturation;
        let l = v.lightness;

        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = l - c / 2.0;

        let (r, g, b) = if h >= 0.0 && h < 60.0 {
            (c, x, 0.0)
        } else if h >= 60.0 && h < 120.0 {
            (x, c, 0.0)
        } else if h >= 120.0 && h < 180.0 {
            (0.0, c, x)
        } else if h >= 180.0 && h < 240.0 {
            (0.0, x, c)
        } else if h >= 240.0 && h < 300.0 {
            (x, 0.0, c)
        } else if h >= 300.0 && h < 360.0 {
            (c, 0.0, x)
        } else {
            (0.0, 0.0, 0.0)
        };

        Color::from(Vector4::new(r + m, g + m, b + m, 1.0))
    }
}

impl From<Color> for Hsl {
    fn from(v: Color) -> Self {
        let f = v.as_frgb();
        let r = f.x;
        let g = f.y;
        let b = f.z;

        let cmax = r.max(g).max(b);
        let cmin = r.min(g).min(b);

        let d = cmax - cmin;

        let h = if d.is_zero() {
            0.0
        } else if cmax.eq(&r) {
            let k = 60.0 * (((g - b) / d) % 6.0);
            if g >= b {
                k
            } else {
                k + 360.0
            }
        } else if cmax.eq(&g) {
            60.0 * ((b - r) / d + 2.0)
        } else if cmax.eq(&b) {
            60.0 * ((r - g) / d + 4.0)
        } else {
            0.0
        };

        let l = (cmax + cmin) / 2.0;

        let s = if d.is_zero() {
            0.0
        } else {
            d / (1.0 - (2.0 * l - 1.0).abs())
        };

        Hsl {
            hue: h,
            saturation: s,
            lightness: l,
        }
    }
}

impl Color {
    pub const WHITE: Self = Self::repeat(255);
    pub const BLACK: Self = Self::opaque(0, 0, 0);
    pub const RED: Self = Self::opaque(255, 0, 0);
    pub const GREEN: Self = Self::opaque(0, 255, 0);
    pub const BLUE: Self = Self::opaque(0, 0, 255);
    pub const TRANSPARENT: Self = Self::repeat(0);
    pub const MAROON: Self = Self::opaque(128, 0, 0);
    pub const DARK_RED: Self = Self::opaque(139, 0, 0);
    pub const BROWN: Self = Self::opaque(165, 42, 42);
    pub const FIREBRICK: Self = Self::opaque(178, 34, 34);
    pub const CRIMSON: Self = Self::opaque(220, 20, 60);
    pub const TOMATO: Self = Self::opaque(255, 99, 71);
    pub const CORAL: Self = Self::opaque(255, 127, 80);
    pub const INDIAN_RED: Self = Self::opaque(205, 92, 92);
    pub const LIGHT_CORAL: Self = Self::opaque(240, 128, 128);
    pub const DARK_SALMON: Self = Self::opaque(233, 150, 122);
    pub const SALMON: Self = Self::opaque(250, 128, 114);
    pub const LIGHT_SALMON: Self = Self::opaque(255, 160, 122);
    pub const ORANGE_RED: Self = Self::opaque(255, 69, 0);
    pub const DARK_ORANGE: Self = Self::opaque(255, 140, 0);
    pub const ORANGE: Self = Self::opaque(255, 165, 0);
    pub const GOLD: Self = Self::opaque(255, 215, 0);
    pub const DARK_GOLDEN_ROD: Self = Self::opaque(184, 134, 11);
    pub const GOLDEN_ROD: Self = Self::opaque(218, 165, 32);
    pub const PALE_GOLDEN_ROD: Self = Self::opaque(238, 232, 170);
    pub const DARK_KHAKI: Self = Self::opaque(189, 183, 107);
    pub const KHAKI: Self = Self::opaque(240, 230, 140);
    pub const OLIVE: Self = Self::opaque(128, 128, 0);
    pub const YELLOW: Self = Self::opaque(255, 255, 0);
    pub const YELLOW_GREEN: Self = Self::opaque(154, 205, 50);
    pub const DARK_OLIVE_GREEN: Self = Self::opaque(85, 107, 47);
    pub const OLIVE_DRAB: Self = Self::opaque(107, 142, 35);
    pub const LAWN_GREEN: Self = Self::opaque(124, 252, 0);
    pub const CHARTREUSE: Self = Self::opaque(127, 255, 0);
    pub const GREEN_YELLOW: Self = Self::opaque(173, 255, 47);
    pub const DARK_GREEN: Self = Self::opaque(0, 100, 0);
    pub const FOREST_GREEN: Self = Self::opaque(34, 139, 34);
    pub const LIME: Self = Self::opaque(0, 255, 0);
    pub const LIME_GREEN: Self = Self::opaque(50, 205, 50);
    pub const LIGHT_GREEN: Self = Self::opaque(144, 238, 144);
    pub const PALE_GREEN: Self = Self::opaque(152, 251, 152);
    pub const DARK_SEA_GREEN: Self = Self::opaque(143, 188, 143);
    pub const MEDIUM_SPRING_GREEN: Self = Self::opaque(0, 250, 154);
    pub const SPRING_GREEN: Self = Self::opaque(0, 255, 127);
    pub const SEA_GREEN: Self = Self::opaque(46, 139, 87);
    pub const MEDIUM_AQUA_MARINE: Self = Self::opaque(102, 205, 170);
    pub const MEDIUM_SEA_GREEN: Self = Self::opaque(60, 179, 113);
    pub const LIGHT_SEA_GREEN: Self = Self::opaque(32, 178, 170);
    pub const DARK_SLATE_GRAY: Self = Self::opaque(47, 79, 79);
    pub const TEAL: Self = Self::opaque(0, 128, 128);
    pub const DARK_CYAN: Self = Self::opaque(0, 139, 139);
    pub const AQUA: Self = Self::opaque(0, 255, 255);
    pub const CYAN: Self = Self::opaque(0, 255, 255);
    pub const LIGHT_CYAN: Self = Self::opaque(224, 255, 255);
    pub const DARK_TURQUOISE: Self = Self::opaque(0, 206, 209);
    pub const TURQUOISE: Self = Self::opaque(64, 224, 208);
    pub const MEDIUM_TURQUOISE: Self = Self::opaque(72, 209, 204);
    pub const PALE_TURQUOISE: Self = Self::opaque(175, 238, 238);
    pub const AQUA_MARINE: Self = Self::opaque(127, 255, 212);
    pub const POWDER_BLUE: Self = Self::opaque(176, 224, 230);
    pub const CADET_BLUE: Self = Self::opaque(95, 158, 160);
    pub const STEEL_BLUE: Self = Self::opaque(70, 130, 180);
    pub const CORN_FLOWER_BLUE: Self = Self::opaque(100, 149, 237);
    pub const DEEP_SKY_BLUE: Self = Self::opaque(0, 191, 255);
    pub const DODGER_BLUE: Self = Self::opaque(30, 144, 255);
    pub const LIGHT_BLUE: Self = Self::opaque(173, 216, 230);
    pub const SKY_BLUE: Self = Self::opaque(135, 206, 235);
    pub const LIGHT_SKY_BLUE: Self = Self::opaque(135, 206, 250);
    pub const MIDNIGHT_BLUE: Self = Self::opaque(25, 25, 112);
    pub const NAVY: Self = Self::opaque(0, 0, 128);
    pub const DARK_BLUE: Self = Self::opaque(0, 0, 139);
    pub const MEDIUM_BLUE: Self = Self::opaque(0, 0, 205);
    pub const ROYAL_BLUE: Self = Self::opaque(65, 105, 225);
    pub const BLUE_VIOLET: Self = Self::opaque(138, 43, 226);
    pub const INDIGO: Self = Self::opaque(75, 0, 130);
    pub const DARK_SLATE_BLUE: Self = Self::opaque(72, 61, 139);
    pub const SLATE_BLUE: Self = Self::opaque(106, 90, 205);
    pub const MEDIUM_SLATE_BLUE: Self = Self::opaque(123, 104, 238);
    pub const MEDIUM_PURPLE: Self = Self::opaque(147, 112, 219);
    pub const DARK_MAGENTA: Self = Self::opaque(139, 0, 139);
    pub const DARK_VIOLET: Self = Self::opaque(148, 0, 211);
    pub const DARK_ORCHID: Self = Self::opaque(153, 50, 204);
    pub const MEDIUM_ORCHID: Self = Self::opaque(186, 85, 211);
    pub const PURPLE: Self = Self::opaque(128, 0, 128);
    pub const THISTLE: Self = Self::opaque(216, 191, 216);
    pub const PLUM: Self = Self::opaque(221, 160, 221);
    pub const VIOLET: Self = Self::opaque(238, 130, 238);
    pub const MAGENTA: Self = Self::opaque(255, 0, 255);
    pub const ORCHID: Self = Self::opaque(218, 112, 214);
    pub const MEDIUM_VIOLET_RED: Self = Self::opaque(199, 21, 133);
    pub const PALE_VIOLET_RED: Self = Self::opaque(219, 112, 147);
    pub const DEEP_PINK: Self = Self::opaque(255, 20, 147);
    pub const HOT_PINK: Self = Self::opaque(255, 105, 180);
    pub const LIGHT_PINK: Self = Self::opaque(255, 182, 193);
    pub const PINK: Self = Self::opaque(255, 192, 203);
    pub const ANTIQUE_WHITE: Self = Self::opaque(250, 235, 215);
    pub const BEIGE: Self = Self::opaque(245, 245, 220);
    pub const BISQUE: Self = Self::opaque(255, 228, 196);
    pub const BLANCHED_ALMOND: Self = Self::opaque(255, 235, 205);
    pub const WHEAT: Self = Self::opaque(245, 222, 179);
    pub const CORN_SILK: Self = Self::opaque(255, 248, 220);
    pub const LEMON_CHIFFON: Self = Self::opaque(255, 250, 205);
    pub const LIGHT_GOLDEN_ROD_YELLOW: Self = Self::opaque(250, 250, 210);
    pub const LIGHT_YELLOW: Self = Self::opaque(255, 255, 224);
    pub const SADDLE_BROWN: Self = Self::opaque(139, 69, 19);
    pub const SIENNA: Self = Self::opaque(160, 82, 45);
    pub const CHOCOLATE: Self = Self::opaque(210, 105, 30);
    pub const PERU: Self = Self::opaque(205, 133, 63);
    pub const SANDY_BROWN: Self = Self::opaque(244, 164, 96);
    pub const BURLY_WOOD: Self = Self::opaque(222, 184, 135);
    pub const TAN: Self = Self::opaque(210, 180, 140);
    pub const ROSY_BROWN: Self = Self::opaque(188, 143, 143);
    pub const MOCCASIN: Self = Self::opaque(255, 228, 181);
    pub const NAVAJO_WHITE: Self = Self::opaque(255, 222, 173);
    pub const PEACH_PUFF: Self = Self::opaque(255, 218, 185);
    pub const MISTY_ROSE: Self = Self::opaque(255, 228, 225);
    pub const LAVENDER_BLUSH: Self = Self::opaque(255, 240, 245);
    pub const LINEN: Self = Self::opaque(250, 240, 230);
    pub const OLD_LACE: Self = Self::opaque(253, 245, 230);
    pub const PAPAYA_WHIP: Self = Self::opaque(255, 239, 213);
    pub const SEA_SHELL: Self = Self::opaque(255, 245, 238);
    pub const MINT_CREAM: Self = Self::opaque(245, 255, 250);
    pub const SLATE_GRAY: Self = Self::opaque(112, 128, 144);
    pub const LIGHT_SLATE_GRAY: Self = Self::opaque(119, 136, 153);
    pub const LIGHT_STEEL_BLUE: Self = Self::opaque(176, 196, 222);
    pub const LAVENDER: Self = Self::opaque(230, 230, 250);
    pub const FLORAL_WHITE: Self = Self::opaque(255, 250, 240);
    pub const ALICE_BLUE: Self = Self::opaque(240, 248, 255);
    pub const GHOST_WHITE: Self = Self::opaque(248, 248, 255);
    pub const HONEYDEW: Self = Self::opaque(240, 255, 240);
    pub const IVORY: Self = Self::opaque(255, 255, 240);
    pub const AZURE: Self = Self::opaque(240, 255, 255);
    pub const SNOW: Self = Self::opaque(255, 250, 250);
    pub const DIM_GRAY: Self = Self::opaque(105, 105, 105);
    pub const GRAY: Self = Self::opaque(128, 128, 128);
    pub const DARK_GRAY: Self = Self::opaque(169, 169, 169);
    pub const SILVER: Self = Self::opaque(192, 192, 192);
    pub const LIGHT_GRAY: Self = Self::opaque(211, 211, 211);
    pub const GAINSBORO: Self = Self::opaque(220, 220, 220);
    pub const WHITE_SMOKE: Self = Self::opaque(245, 245, 245);

    pub const COLORS: [Self; 140] = [
        Self::TRANSPARENT,
        Self::WHITE,
        Self::BLACK,
        Self::RED,
        Self::GREEN,
        Self::BLUE,
        Self::MAROON,
        Self::DARK_RED,
        Self::BROWN,
        Self::FIREBRICK,
        Self::CRIMSON,
        Self::TOMATO,
        Self::CORAL,
        Self::INDIAN_RED,
        Self::LIGHT_CORAL,
        Self::DARK_SALMON,
        Self::SALMON,
        Self::LIGHT_SALMON,
        Self::ORANGE_RED,
        Self::DARK_ORANGE,
        Self::ORANGE,
        Self::GOLD,
        Self::DARK_GOLDEN_ROD,
        Self::GOLDEN_ROD,
        Self::PALE_GOLDEN_ROD,
        Self::DARK_KHAKI,
        Self::KHAKI,
        Self::OLIVE,
        Self::YELLOW,
        Self::YELLOW_GREEN,
        Self::DARK_OLIVE_GREEN,
        Self::OLIVE_DRAB,
        Self::LAWN_GREEN,
        Self::CHARTREUSE,
        Self::GREEN_YELLOW,
        Self::DARK_GREEN,
        Self::FOREST_GREEN,
        Self::LIME,
        Self::LIME_GREEN,
        Self::LIGHT_GREEN,
        Self::PALE_GREEN,
        Self::DARK_SEA_GREEN,
        Self::MEDIUM_SPRING_GREEN,
        Self::SPRING_GREEN,
        Self::SEA_GREEN,
        Self::MEDIUM_AQUA_MARINE,
        Self::MEDIUM_SEA_GREEN,
        Self::LIGHT_SEA_GREEN,
        Self::DARK_SLATE_GRAY,
        Self::TEAL,
        Self::DARK_CYAN,
        Self::AQUA,
        Self::CYAN,
        Self::LIGHT_CYAN,
        Self::DARK_TURQUOISE,
        Self::TURQUOISE,
        Self::MEDIUM_TURQUOISE,
        Self::PALE_TURQUOISE,
        Self::AQUA_MARINE,
        Self::POWDER_BLUE,
        Self::CADET_BLUE,
        Self::STEEL_BLUE,
        Self::CORN_FLOWER_BLUE,
        Self::DEEP_SKY_BLUE,
        Self::DODGER_BLUE,
        Self::LIGHT_BLUE,
        Self::SKY_BLUE,
        Self::LIGHT_SKY_BLUE,
        Self::MIDNIGHT_BLUE,
        Self::NAVY,
        Self::DARK_BLUE,
        Self::MEDIUM_BLUE,
        Self::ROYAL_BLUE,
        Self::BLUE_VIOLET,
        Self::INDIGO,
        Self::DARK_SLATE_BLUE,
        Self::SLATE_BLUE,
        Self::MEDIUM_SLATE_BLUE,
        Self::MEDIUM_PURPLE,
        Self::DARK_MAGENTA,
        Self::DARK_VIOLET,
        Self::DARK_ORCHID,
        Self::MEDIUM_ORCHID,
        Self::PURPLE,
        Self::THISTLE,
        Self::PLUM,
        Self::VIOLET,
        Self::MAGENTA,
        Self::ORCHID,
        Self::MEDIUM_VIOLET_RED,
        Self::PALE_VIOLET_RED,
        Self::DEEP_PINK,
        Self::HOT_PINK,
        Self::LIGHT_PINK,
        Self::PINK,
        Self::ANTIQUE_WHITE,
        Self::BEIGE,
        Self::BISQUE,
        Self::BLANCHED_ALMOND,
        Self::WHEAT,
        Self::CORN_SILK,
        Self::LEMON_CHIFFON,
        Self::LIGHT_GOLDEN_ROD_YELLOW,
        Self::LIGHT_YELLOW,
        Self::SADDLE_BROWN,
        Self::SIENNA,
        Self::CHOCOLATE,
        Self::PERU,
        Self::SANDY_BROWN,
        Self::BURLY_WOOD,
        Self::TAN,
        Self::ROSY_BROWN,
        Self::MOCCASIN,
        Self::NAVAJO_WHITE,
        Self::PEACH_PUFF,
        Self::MISTY_ROSE,
        Self::LAVENDER_BLUSH,
        Self::LINEN,
        Self::OLD_LACE,
        Self::PAPAYA_WHIP,
        Self::SEA_SHELL,
        Self::MINT_CREAM,
        Self::SLATE_GRAY,
        Self::LIGHT_SLATE_GRAY,
        Self::LIGHT_STEEL_BLUE,
        Self::LAVENDER,
        Self::FLORAL_WHITE,
        Self::ALICE_BLUE,
        Self::GHOST_WHITE,
        Self::HONEYDEW,
        Self::IVORY,
        Self::AZURE,
        Self::SNOW,
        Self::DIM_GRAY,
        Self::GRAY,
        Self::DARK_GRAY,
        Self::SILVER,
        Self::LIGHT_GRAY,
        Self::GAINSBORO,
        Self::WHITE_SMOKE,
    ];

    #[inline]
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    #[inline]
    pub const fn repeat(c: u8) -> Self {
        Self {
            r: c,
            g: c,
            b: c,
            a: c,
        }
    }

    #[inline]
    pub const fn repeat_opaque(c: u8) -> Self {
        Self {
            r: c,
            g: c,
            b: c,
            a: 255,
        }
    }

    #[inline]
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[must_use]
    #[inline]
    pub fn srgb_to_linear(self) -> Self {
        let r = ((self.r as f32 / 255.0).powf(2.2).clamp(0.0, 1.0) * 255.0) as u8;
        let g = ((self.g as f32 / 255.0).powf(2.2).clamp(0.0, 1.0) * 255.0) as u8;
        let b = ((self.b as f32 / 255.0).powf(2.2).clamp(0.0, 1.0) * 255.0) as u8;
        Self::from_rgba(r, g, b, self.a)
    }

    #[must_use]
    #[inline]
    pub fn srgb_to_linear_f32(self) -> Vector4<f32> {
        let r = (self.r as f32 / 255.0).powf(2.2).clamp(0.0, 1.0);
        let g = (self.g as f32 / 255.0).powf(2.2).clamp(0.0, 1.0);
        let b = (self.b as f32 / 255.0).powf(2.2).clamp(0.0, 1.0);
        Vector4::new(r, g, b, self.a as f32 / 255.0)
    }

    #[must_use]
    #[inline]
    pub fn linear_to_srgb(self) -> Self {
        let r = ((self.r as f32 / 255.0).powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8;
        let g = ((self.g as f32 / 255.0).powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8;
        let b = ((self.b as f32 / 255.0).powf(1.0 / 2.2).clamp(0.0, 1.0) * 255.0) as u8;
        Self::from_rgba(r, g, b, self.a)
    }

    #[inline]
    pub fn as_frgba(self) -> Vector4<f32> {
        Vector4::new(
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
            f32::from(self.a) / 255.0,
        )
    }

    #[inline]
    pub fn as_frgb(self) -> Vector3<f32> {
        Vector3::new(
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
        )
    }

    #[inline]
    pub fn to_opaque(self) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: 255,
        }
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let dr = (t * (i32::from(other.r) - i32::from(self.r)) as f32) as i32;
        let dg = (t * (i32::from(other.g) - i32::from(self.g)) as f32) as i32;
        let db = (t * (i32::from(other.b) - i32::from(self.b)) as f32) as i32;
        let da = (t * (i32::from(other.a) - i32::from(self.a)) as f32) as i32;

        let red = (i32::from(self.r) + dr) as u8;
        let green = (i32::from(self.g) + dg) as u8;
        let blue = (i32::from(self.b) + db) as u8;
        let alpha = (i32::from(self.a) + da) as u8;

        Self {
            r: red,
            g: green,
            b: blue,
            a: alpha,
        }
    }

    #[inline]
    pub fn with_new_alpha(self, a: u8) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a,
        }
    }
}

impl Add for Color {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            r: self.r.saturating_add(rhs.r),
            g: self.g.saturating_add(rhs.g),
            b: self.b.saturating_add(rhs.b),
            a: self.a.saturating_add(rhs.a),
        }
    }
}

impl AddAssign for Color {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Color {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            r: self.r.saturating_sub(rhs.r),
            g: self.g.saturating_sub(rhs.g),
            b: self.b.saturating_sub(rhs.b),
            a: self.a.saturating_sub(rhs.a),
        }
    }
}

impl SubAssign for Color {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
mod test {
    use crate::algebra::{Vector3, Vector4};
    use crate::color::{Color, Hsl, Hsv};

    #[test]
    fn test_hsl() {
        // Hsl -> Rgb
        assert_eq!(Color::from(Hsl::new(0.0, 0.0, 0.0)), Color::opaque(0, 0, 0));
        assert_eq!(
            Color::from(Hsl::new(0.0, 0.0, 1.0)),
            Color::opaque(255, 255, 255)
        );
        assert_eq!(
            Color::from(Hsl::new(0.0, 1.0, 0.5)),
            Color::opaque(255, 0, 0)
        );
        assert_eq!(
            Color::from(Hsl::new(120.0, 1.0, 0.5)),
            Color::opaque(0, 255, 0)
        );
        assert_eq!(
            Color::from(Hsl::new(240.0, 1.0, 0.5)),
            Color::opaque(0, 0, 255)
        );
        assert_eq!(
            Color::from(Hsl::new(60.0, 1.0, 0.5)),
            Color::opaque(255, 255, 0)
        );
        assert_eq!(
            Color::from(Hsl::new(180.0, 1.0, 0.5)),
            Color::opaque(0, 255, 255)
        );
        assert_eq!(
            Color::from(Hsl::new(300.0, 1.0, 0.5)),
            Color::opaque(255, 0, 255)
        );
        assert_eq!(
            Color::from(Hsl::new(0.0, 0.0, 0.75)),
            Color::opaque(191, 191, 191)
        );

        // Rgb -> Hsl
        assert_eq!(Hsl::from(Color::opaque(0, 0, 0)), Hsl::new(0.0, 0.0, 0.0));
        assert_eq!(
            Hsl::from(Color::opaque(255, 255, 255)),
            Hsl::new(0.0, 0.0, 1.0)
        );
        assert_eq!(Hsl::from(Color::opaque(255, 0, 0)), Hsl::new(0.0, 1.0, 0.5));
        assert_eq!(
            Hsl::from(Color::opaque(0, 255, 0)),
            Hsl::new(120.0, 1.0, 0.5)
        );
        assert_eq!(
            Hsl::from(Color::opaque(0, 0, 255)),
            Hsl::new(240.0, 1.0, 0.5)
        );
        assert_eq!(
            Hsl::from(Color::opaque(255, 255, 0)),
            Hsl::new(60.0, 1.0, 0.5)
        );
        assert_eq!(
            Hsl::from(Color::opaque(0, 255, 255)),
            Hsl::new(180.0, 1.0, 0.5)
        );
        assert_eq!(
            Hsl::from(Color::opaque(255, 0, 255)),
            Hsl::new(300.0, 1.0, 0.5)
        );
        assert_eq!(
            Hsl::from(Color::opaque(191, 191, 191)),
            Hsl::new(0.0, 0.0, 0.7490196)
        );

        let mut color = Hsl::new(0.0, 0.0, 0.0);
        assert_eq!(color.hue(), 0.0);
        assert_eq!(color.saturation(), 0.0);
        assert_eq!(color.lightness(), 0.0);

        color.set_hue(370.0);
        color.set_saturation(2.0);
        color.set_lightness(2.0);
        assert_eq!(color.hue(), 10.0);
        assert_eq!(color.saturation(), 1.0);
        assert_eq!(color.lightness(), 1.0);
    }

    #[test]
    fn test_color_default() {
        assert_eq!(Color::default(), Color::WHITE);
    }

    #[test]
    fn test_color_into_u32() {
        let black: u32 = Color::BLACK.into();
        assert_eq!(black, 0xFF000000);

        let white: u32 = Color::WHITE.into();
        assert_eq!(white, 0xFFFFFFFF);

        let red: u32 = Color::RED.into();
        assert_eq!(red, 0xFF0000FF);

        let green: u32 = Color::GREEN.into();
        assert_eq!(green, 0xFF00FF00);

        let blue: u32 = Color::BLUE.into();
        assert_eq!(blue, 0xFFFF0000);
    }

    #[test]
    fn test_color_from_vector3() {
        assert_eq!(Color::from(Vector3::new(0_f32, 0_f32, 0_f32)), Color::BLACK);
        assert_eq!(
            Color::from(Vector3::new(255_f32, 255_f32, 255_f32)),
            Color::WHITE
        );
        assert_eq!(Color::from(Vector3::new(255_f32, 0_f32, 0_f32)), Color::RED);
        assert_eq!(
            Color::from(Vector3::new(0_f32, 255_f32, 0_f32)),
            Color::GREEN
        );
        assert_eq!(
            Color::from(Vector3::new(0_f32, 0_f32, 255_f32)),
            Color::BLUE
        );
    }

    #[test]
    fn test_hsv() {
        assert_eq!(
            Hsv::new(0.0, 0.0, 0.0),
            Hsv {
                hue: 0.0,
                saturation: 0.0,
                brightness: 0.0
            }
        );

        assert_eq!(
            Hsv::new(1000.0, 1000.0, 1000.0),
            Hsv {
                hue: 360.0,
                saturation: 100.0,
                brightness: 100.0
            }
        );

        let mut color = Hsv::new(0.0, 0.0, 0.0);
        assert_eq!(color.hue(), 0.0);
        assert_eq!(color.saturation(), 0.0);
        assert_eq!(color.brightness(), 0.0);

        color.set_hue(1000.0);
        color.set_saturation(1000.0);
        color.set_brightness(1000.0);
        assert_eq!(color.hue(), 360.0);
        assert_eq!(color.saturation(), 100.0);
        assert_eq!(color.brightness(), 100.0);
    }

    #[test]
    fn test_hsv_from_color() {
        let black = Hsv::new(0.0, 0.0, 0.0);
        assert_eq!(Hsv::from(Color::BLACK), black);

        let white = Hsv::new(0.0, 0.0, 100.0);
        assert_eq!(Hsv::from(Color::WHITE), white);

        let red = Hsv::new(0.0, 100.0, 100.0);
        assert_eq!(Hsv::from(Color::RED), red);

        let green = Hsv::new(120.0, 100.0, 100.0);
        assert_eq!(Hsv::from(Color::GREEN), green);

        let blue = Hsv::new(240.0, 100.0, 100.0);
        assert_eq!(Hsv::from(Color::BLUE), blue);

        let color = Hsv::new(300.0, 100.0, 100.0);
        assert_eq!(Hsv::from(Color::opaque(255, 0, 255)), color);
    }

    #[test]
    fn test_color_from_hsv() {
        let black = Hsv::new(0.0, 0.0, 0.0);
        assert_eq!(Color::from(black), Color::BLACK);

        let white = Hsv::new(0.0, 0.0, 100.0);
        assert_eq!(Color::from(white), Color::WHITE);

        let red = Hsv::new(0.0, 100.0, 100.0);
        assert_eq!(Color::from(red), Color::RED);

        let green = Hsv::new(120.0, 100.0, 100.0);
        assert_eq!(Color::from(green), Color::GREEN);

        let blue = Hsv::new(240.0, 100.0, 100.0);
        assert_eq!(Color::from(blue), Color::BLUE);

        let color = Hsv::new(300.0, 100.0, 100.0);
        assert_eq!(Color::from(color), Color::opaque(255, 0, 255));

        let color = Hsv::new(60.0, 0.0, 10.0);
        assert_eq!(Color::from(color), Color::opaque(25, 25, 25));

        let color = Hsv::new(180.0, 100.0, 100.0);
        assert_eq!(Color::from(color), Color::opaque(0, 255, 255));
    }

    #[test]
    fn test_color_from_rgba() {
        assert_eq!(
            Color::from_rgba(0, 0, 0, 0),
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0
            }
        );
        assert_eq!(Color::from_rgba(0, 0, 0, 255), Color::BLACK);
    }

    #[test]
    fn test_color_srgb_to_linear() {
        assert_eq!(Color::BLACK.srgb_to_linear(), Color::BLACK);
        assert_eq!(Color::WHITE.srgb_to_linear(), Color::WHITE);

        let color = Color::opaque(100, 100, 100);
        assert_eq!(color.srgb_to_linear(), Color::opaque(32, 32, 32));
    }

    #[test]
    fn test_color_srgb_to_linear_f32() {
        assert_eq!(
            Color::BLACK.srgb_to_linear_f32(),
            Vector4::new(0.0, 0.0, 0.0, 1.0)
        );
        assert_eq!(
            Color::WHITE.srgb_to_linear_f32(),
            Vector4::new(1.0, 1.0, 1.0, 1.0)
        );

        let color = Color::opaque(200, 200, 200);
        assert_eq!(
            color.srgb_to_linear_f32(),
            Vector4::new(0.585_973, 0.585_973, 0.585_973, 1.0)
        );
    }

    #[test]
    fn test_color_linear_to_srgb() {
        assert_eq!(Color::BLACK.linear_to_srgb(), Color::BLACK);
        assert_eq!(Color::WHITE.linear_to_srgb(), Color::WHITE);

        let color = Color::opaque(32, 32, 32);
        assert_eq!(color.linear_to_srgb(), Color::opaque(99, 99, 99));
    }

    #[test]
    fn test_color_as_frgba() {
        assert_eq!(Color::BLACK.as_frgba(), Vector4::new(0.0, 0.0, 0.0, 1.0));
        assert_eq!(Color::WHITE.as_frgba(), Vector4::new(1.0, 1.0, 1.0, 1.0));

        let color = Color::opaque(100, 100, 100);
        assert_eq!(
            color.as_frgba(),
            Vector4::new(0.39215687, 0.39215687, 0.39215687, 1.0)
        );
    }

    #[test]
    fn test_color_to_opaque() {
        assert_eq!(Color::BLACK.to_opaque(), Color::BLACK);
        assert_eq!(Color::TRANSPARENT.to_opaque(), Color::BLACK);
    }

    #[test]
    fn test_color_lerp() {
        let color = Color::BLACK.lerp(Color::WHITE, 0.5);
        assert_eq!(color, Color::opaque(127, 127, 127));
    }

    #[test]
    fn test_color_with_new_alpha() {
        let color = Color::BLACK;
        assert_eq!(color.with_new_alpha(0), Color::TRANSPARENT);
    }

    #[test]
    fn test_color_operators() {
        assert_eq!(Color::RED + Color::GREEN + Color::BLUE, Color::WHITE);
        assert_eq!(
            Color::WHITE - Color::RED - Color::GREEN - Color::BLUE,
            Color::TRANSPARENT
        );

        let mut color = Color::opaque(100, 100, 100);
        color += Color::opaque(155, 155, 155);
        assert_eq!(color, Color::WHITE);
        color -= Color::opaque(155, 155, 155);
        assert_eq!(color, Color::from_rgba(100, 100, 100, 0));
    }
}
