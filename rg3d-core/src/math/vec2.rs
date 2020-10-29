#![allow(clippy::len_without_is_empty)]

use std::hash::{Hash, Hasher};
use std::ops;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl PartialEq for Vec2 {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Eq for Vec2 {}

impl Hash for Vec2 {
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

impl Default for Vec2 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const UNIT: Self = Self { x: 1.0, y: 1.0 };

    pub const X: Self = Self { x: 1.0, y: 0.0 };
    pub const Y: Self = Self { x: 0.0, y: 1.0 };

    fn validate(self) {
        debug_assert!(!self.x.is_nan());
        debug_assert!(!self.y.is_nan());
    }

    #[inline]
    pub const fn xx(self) -> Self {
        Self {
            x: self.x,
            y: self.x,
        }
    }

    #[inline]
    pub const fn yy(self) -> Self {
        Self {
            x: self.y,
            y: self.y,
        }
    }

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }

    #[inline]
    pub fn angle(self, other: Self) -> f32 {
        (self.dot(other) / (self.len() * other.len())).acos()
    }

    #[inline]
    pub fn perpendicular(self) -> Self {
        Self {
            x: self.y,
            y: -self.x,
        }
    }

    #[inline]
    pub fn scale(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }

    /// Per component min
    pub fn min(&self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    /// Per component max
    pub fn max(&self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    #[inline]
    pub fn normalized(self) -> Option<Self> {
        let len = self.len();
        if len >= std::f32::EPSILON {
            let inv_len = 1.0 / len;
            return Some(Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
            });
        }
        None
    }

    #[inline]
    pub fn reciprocal(&self) -> Vec2 {
        Vec2::new(1.0 / self.x, 1.0 / self.y)
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from(v: (f32, f32)) -> Self {
        Self { x: v.0, y: v.1 }
    }
}

impl From<Vec2> for (f32, f32) {
    fn from(v: Vec2) -> Self {
        (v.x, v.y)
    }
}

impl ops::Add<Self> for Vec2 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self {
            x: self.x + b.x,
            y: self.y + b.y,
        }
    }
}

impl ops::AddAssign<Self> for Vec2 {
    fn add_assign(&mut self, b: Self) {
        self.x += b.x;
        self.y += b.y;
    }
}

impl ops::Sub<Self> for Vec2 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self {
            x: self.x - b.x,
            y: self.y - b.y,
        }
    }
}

impl ops::SubAssign<Self> for Vec2 {
    fn sub_assign(&mut self, b: Self) {
        self.x -= b.x;
        self.y -= b.y;
    }
}

impl ops::Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}
