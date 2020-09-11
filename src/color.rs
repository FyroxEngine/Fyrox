use crate::math::vec3::Vec3;
use crate::{
    math::vec4::Vec4,
    visitor::{Visit, VisitResult, Visitor},
};

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
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }
}

impl From<Vec3> for Color {
    fn from(v: Vec3) -> Self {
        Self {
            r: (v.x.max(0.0).min(1.0) * 255.0) as u8,
            g: (v.y.max(0.0).min(1.0) * 255.0) as u8,
            b: (v.z.max(0.0).min(1.0) * 255.0) as u8,
            a: 255,
        }
    }
}

impl From<Vec4> for Color {
    fn from(v: Vec4) -> Self {
        Self {
            r: (v.x.max(0.0).min(1.0) * 255.0) as u8,
            g: (v.y.max(0.0).min(1.0) * 255.0) as u8,
            b: (v.z.max(0.0).min(1.0) * 255.0) as u8,
            a: (v.w.max(0.0).min(1.0) * 255.0) as u8,
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

    pub const fn opaque(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    pub fn as_frgba(self) -> Vec4 {
        Vec4 {
            x: f32::from(self.r) / 255.0,
            y: f32::from(self.g) / 255.0,
            z: f32::from(self.b) / 255.0,
            w: f32::from(self.a) / 255.0,
        }
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
