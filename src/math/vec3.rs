#![allow(clippy::len_without_is_empty)]

use crate::{math::lerpf, math::vec2::Vec2};
use std::{
    hash::{Hash, Hasher},
    ops,
};

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl PartialEq for Vec3 {
    fn eq(&self, other: &Self) -> bool {
        self.validate();
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

impl Eq for Vec3 {}

impl Hash for Vec3 {
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

impl Default for Vec3 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    pub const UNIT: Self = Self {
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    pub const X: Self = Self {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    pub const Y: Self = Self {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    pub const Z: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    pub const RIGHT: Self = Self::X;
    pub const UP: Self = Self::Y;
    pub const LOOK: Self = Self::Z;

    fn validate(&self) {
        debug_assert!(!self.x.is_nan());
        debug_assert!(!self.y.is_nan());
        debug_assert!(!self.z.is_nan());
    }

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn xy(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    pub const fn xz(&self) -> Vec2 {
        Vec2::new(self.x, self.z)
    }

    pub const fn zx(&self) -> Vec2 {
        Vec2::new(self.z, self.x)
    }

    pub const fn yx(&self) -> Vec2 {
        Vec2::new(self.y, self.x)
    }

    pub const fn zy(&self) -> Vec2 {
        Vec2::new(self.z, self.y)
    }

    pub const fn yz(&self) -> Vec2 {
        Vec2::new(self.y, self.z)
    }

    #[inline]
    pub fn scale(&self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    #[inline]
    pub fn sqr_len(&self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn len(&self) -> f32 {
        self.sqr_len().sqrt()
    }

    #[inline]
    pub fn dot(&self, b: &Self) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }

    #[inline]
    pub fn cross(&self, b: &Self) -> Self {
        Self {
            x: self.y * b.z - self.z * b.y,
            y: self.z * b.x - self.x * b.z,
            z: self.x * b.y - self.y * b.x,
        }
    }

    #[inline]
    pub fn normalized(&self) -> Option<Self> {
        let len = self.len();
        if len >= std::f32::EPSILON {
            let inv_len = 1.0 / len;
            return Some(Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            });
        }
        None
    }

    /// Returns normalized vector and its original length. May fail if vector is
    /// degenerate.
    #[inline]
    pub fn normalized_ex(&self) -> (Option<Self>, f32) {
        let len = self.len();

        let normalized = if len >= std::f32::EPSILON {
            let inv_len = 1.0 / len;
            Some(Self {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            })
        } else {
            None
        };

        (normalized, len)
    }

    #[inline]
    pub fn normalized_unchecked(&self) -> Self {
        let inv_len = 1.0 / self.len();
        Self {
            x: self.x * inv_len,
            y: self.y * inv_len,
            z: self.z * inv_len,
        }
    }

    #[inline]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        Self {
            x: lerpf(self.x, other.x, t),
            y: lerpf(self.y, other.y, t),
            z: lerpf(self.z, other.z, t),
        }
    }

    #[inline]
    pub fn distance(&self, other: &Self) -> f32 {
        (*self - *other).len()
    }

    #[inline]
    pub fn sqr_distance(&self, other: &Self) -> f32 {
        (*self - *other).sqr_len()
    }

    #[inline]
    pub fn is_same_direction_as(&self, other: &Self) -> bool {
        self.dot(other) > 0.0
    }

    #[inline]
    pub fn min_value(&self) -> f32 {
        self.x.min(self.y).min(self.z)
    }

    #[inline]
    pub fn max_value(&self) -> f32 {
        self.x.max(self.y).max(self.z)
    }

    pub fn follow(&mut self, other: &Self, fraction: f32) {
        self.x += (other.x - self.x) * fraction;
        self.y += (other.y - self.y) * fraction;
        self.z += (other.z - self.z) * fraction;
    }

    pub fn project(&self, normal: &Self) -> Self {
        let n = normal.normalized().unwrap();
        *self - n.scale(self.dot(&n))
    }
}

impl ops::Add<Self> for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, b: Self) -> Self {
        Self {
            x: self.x + b.x,
            y: self.y + b.y,
            z: self.z + b.z,
        }
    }
}

impl ops::AddAssign<Self> for Vec3 {
    #[inline]
    fn add_assign(&mut self, b: Self) {
        self.x += b.x;
        self.y += b.y;
        self.z += b.z;
    }
}

impl ops::Mul<Self> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: Vec3) -> Self::Output {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }
}

impl ops::Sub<Self> for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, b: Self) -> Self {
        Self {
            x: self.x - b.x,
            y: self.y - b.y,
            z: self.z - b.z,
        }
    }
}

impl ops::SubAssign<Self> for Vec3 {
    #[inline]
    fn sub_assign(&mut self, b: Self) {
        self.x -= b.x;
        self.y -= b.y;
        self.z -= b.z;
    }
}

impl ops::Neg for Vec3 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl ops::Index<usize> for Vec3 {
    type Output = f32;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.y,
            _ => panic!("Invalid index {:?} for Vec3", index),
        }
    }
}
