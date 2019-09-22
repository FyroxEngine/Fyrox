use std::ops;
use crate::math::lerpf;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Vec3 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl Vec3 {
    #[inline]
    pub fn new() -> Self {
        Vec3::default()
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

        let normalized =
            if len >= std::f32::EPSILON {
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
            z: -self.z
        }
    }
}
