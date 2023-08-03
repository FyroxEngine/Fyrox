use crate::{
    algebra::{Vector3, Vector4},
    reflect::prelude::*,
    visitor::{Visit, VisitResult, Visitor},
};
use num_traits::Zero;
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Eq, Visit, Reflect)]
#[repr(C)]
pub struct Color {
    // Do not change order! OpenGL requires this order!
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

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

#[derive(Debug, Copy, Clone, PartialEq)]
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
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const RED: Self = Self {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GREEN: Self = Self {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const BLUE: Self = Self {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const ORANGE: Self = Self {
        r: 255,
        g: 69,
        b: 0,
        a: 255,
    };

    #[inline]
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
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
