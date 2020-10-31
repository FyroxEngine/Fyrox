use crate::math::{vec2::Vec2, vec3::Vec3};
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Debug)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl PartialEq for Vec4 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z && self.w == other.w
    }
}

impl Eq for Vec4 {}

impl Hash for Vec4 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.validate();
        unsafe {
            state.write(std::slice::from_raw_parts(
                self as *const Self as *const _,
                std::mem::size_of::<Self>(),
            ))
        }
    }
}

impl From<(f32, f32, f32, f32)> for Vec4 {
    fn from(v: (f32, f32, f32, f32)) -> Self {
        Self {
            x: v.0,
            y: v.1,
            z: v.2,
            w: v.3,
        }
    }
}

impl Into<(f32, f32, f32, f32)> for Vec4 {
    fn into(self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.z, self.w)
    }
}

impl Vec4 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 0.0,
    };

    pub const fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    fn validate(&self) {
        debug_assert!(!self.x.is_nan());
        debug_assert!(!self.y.is_nan());
        debug_assert!(!self.z.is_nan());
        debug_assert!(!self.w.is_nan());
    }

    pub const fn from_vec3(v: Vec3, w: f32) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            w,
        }
    }

    pub const fn xyz(&self) -> Vec3 {
        Vec3 {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }

    pub const fn xy(&self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }

    #[inline]
    pub fn reciprocal(&self) -> Vec4 {
        Vec4::new(1.0 / self.x, 1.0 / self.y, 1.0 / self.z, 1.0 / self.w)
    }
}

impl Default for Vec4 {
    fn default() -> Self {
        Self::ZERO
    }
}
