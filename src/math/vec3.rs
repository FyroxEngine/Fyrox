use std::ops;
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    #[inline]
    pub fn new() -> Self {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[inline]
    pub fn make(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    #[inline]
    pub fn zero() -> Self {
        Vec3 { x: 0.0, y: 0.0, z: 0.0 }
    }

    #[inline]
    pub fn unit() -> Self {
        Vec3 { x: 1.0, y: 1.0, z: 1.0 }
    }

    #[inline]
    pub fn up() -> Self {
        Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }

    #[inline]
    pub fn scale(&self, scalar: f32) -> Self {
        Vec3 {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    #[inline]
    pub fn right() -> Self {
        Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[inline]
    pub fn sqr_len(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
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
    pub fn normalized(&self) -> Option<Vec3> {
        let len = self.len();
        if len >= std::f32::EPSILON {
            let inv_len = 1.0 / len;
            return Some(Vec3 {
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
    pub fn normalized_ex(&self) -> (Option<Vec3>, f32) {
        let len = self.len();

        let normalized =
            if len >= std::f32::EPSILON {
                let inv_len = 1.0 / len;
                Some(Vec3 {
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
    pub fn normalized_unchecked(&self) -> Vec3 {
        let inv_len = 1.0 / self.len();
        Vec3 {
            x: self.x * inv_len,
            y: self.y * inv_len,
            z: self.z * inv_len,
        }
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

