use crate::algebra::{Vector3, Vector4};
use crate::visitor::{Visit, VisitResult, Visitor};

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
#[repr(C)]
pub struct Color {
    // Do not change order! OpenGL requires this order!
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl Into<u32> for Color {
    fn into(self) -> u32 {
        ((self.a as u32) << 24) | ((self.b as u32) << 16) | ((self.g as u32) << 8) | (self.r as u32)
    }
}

impl From<Vector3<f32>> for Color {
    fn from(v: Vector3<f32>) -> Self {
        Self {
            r: (v.x.max(0.0).min(1.0) * 255.0) as u8,
            g: (v.y.max(0.0).min(1.0) * 255.0) as u8,
            b: (v.z.max(0.0).min(1.0) * 255.0) as u8,
            a: 255,
        }
    }
}

impl From<Vector4<f32>> for Color {
    fn from(v: Vector4<f32>) -> Self {
        Self {
            r: (v.x.max(0.0).min(1.0) * 255.0) as u8,
            g: (v.y.max(0.0).min(1.0) * 255.0) as u8,
            b: (v.z.max(0.0).min(1.0) * 255.0) as u8,
            a: (v.w.max(0.0).min(1.0) * 255.0) as u8,
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
    pub fn new(hue: f32, saturation: f32, brightness: f32) -> Self {
        Self {
            hue: hue.min(360.0).max(0.0),
            saturation: saturation.min(100.0).max(0.0),
            brightness: brightness.min(100.0).max(0.0),
        }
    }

    pub fn hue(&self) -> f32 {
        self.hue
    }

    pub fn set_hue(&mut self, hue: f32) {
        self.hue = hue.min(360.0).max(0.0);
    }

    pub fn saturation(&self) -> f32 {
        self.saturation
    }

    pub fn set_saturation(&mut self, saturation: f32) {
        self.saturation = saturation.min(100.0).max(0.0);
    }

    pub fn brightness(&self) -> f32 {
        self.brightness
    }

    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness = brightness.min(100.0).max(0.0);
    }
}

impl From<Color> for Hsv {
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

    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn as_frgba(self) -> Vector4<f32> {
        Vector4::new(
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
            f32::from(self.a) / 255.0,
        )
    }

    pub fn as_frgb(self) -> Vector3<f32> {
        Vector3::new(
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
        )
    }

    pub fn to_opaque(self) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: 255,
        }
    }

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
}

impl Visit for Color {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.r.visit("R", visitor)?;
        self.g.visit("G", visitor)?;
        self.b.visit("B", visitor)?;
        self.a.visit("A", visitor)?;

        visitor.leave_region()
    }
}
