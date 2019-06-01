use std::ops;
use super::mat4::*;

#[derive(Copy, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new() -> Self {
        Vec3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn make(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub fn zero() -> Self {
        Vec3 { x: 0.0, y: 0.0, z: 0.0 }
    }

    pub fn unit() -> Self {
        Vec3 { x: 1.0, y: 1.0, z: 1.0 }
    }

    pub fn up() -> Self {
        Vec3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }


    pub fn right() -> Self {
        Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn sqr_len(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn len(&self) -> f32 {
        self.sqr_len().sqrt()
    }

    pub fn dot(&self, b: &Self) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }

    pub fn cross(&self, b: &Self) -> Self {
        Self {
            x: self.y * b.z - self.z * b.y,
            y: self.z * b.x - self.x * b.z,
            z: self.x * b.y - self.y * b.x,
        }
    }

    pub fn normalized(&self) -> Result<Vec3, &'static str> {
        let len = self.len();
        if len >= 0.000001 {
            let inv_len = 1.0 / len;
            return Ok(Vec3 {
                x: self.x * inv_len,
                y: self.y * inv_len,
                z: self.z * inv_len,
            });
        }
        Err("unable to normalize vector with zero length")
    }

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
    fn add(self, b: Self) -> Self {
        Self {
            x: self.x + b.x,
            y: self.y + b.y,
            z: self.z + b.z,
        }
    }
}

impl ops::AddAssign<Self> for Vec3 {
    fn add_assign(&mut self, b: Self) {
        self.x += b.x;
        self.y += b.y;
        self.z += b.z;
    }
}

impl ops::Sub<Self> for Vec3 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self {
            x: self.x - b.x,
            y: self.y - b.y,
            z: self.z - b.z,
        }
    }
}

impl ops::SubAssign<Self> for Vec3 {
    fn sub_assign(&mut self, b: Self) {
        self.x -= b.x;
        self.y -= b.y;
        self.z -= b.z;
    }
}