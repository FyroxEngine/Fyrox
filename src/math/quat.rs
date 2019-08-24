use crate::math::vec3::*;
use std::ops;

#[derive(Copy, Clone)]
pub struct Quat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

pub enum RotationOrder {
    XYZ,
    XZY,
    YZX,
    YXZ,
    ZXY,
    ZYX,
}

impl Default for Quat {
    fn default() -> Self {
        Self::new()
    }
}

impl Quat {
    pub fn new() -> Self {
        Quat {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    pub fn sqr_len(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w
    }

    pub fn len(&self) -> f32 {
        self.sqr_len().sqrt()
    }

    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Self {
        let half_angle = angle * 0.5;
        let d = axis.len();
        let s = half_angle.sin() / d;
        Self {
            x: axis.x * s,
            y: axis.y * s,
            z: axis.z * s,
            w: half_angle.cos(),
        }
    }

    pub fn from_euler(euler_radians: Vec3, order: RotationOrder) -> Self {
        let qx: Quat = Quat::from_axis_angle(Vec3::make(1.0, 0.0, 0.0), euler_radians.x);
        let qy: Quat = Quat::from_axis_angle(Vec3::make(0.0, 1.0, 0.0), euler_radians.y);
        let qz: Quat = Quat::from_axis_angle(Vec3::make(0.0, 0.0, 1.0), euler_radians.z);
        match order {
            RotationOrder::XYZ => qz * qy * qx,
            RotationOrder::XZY => qy * qz * qx,
            RotationOrder::YZX => qx * qz * qy,
            RotationOrder::YXZ => qz * qx * qy,
            RotationOrder::ZXY => qy * qx * qz,
            RotationOrder::ZYX => qx * qy * qz,
        }
    }

    pub fn dot(&self, other: &Quat) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z + self.w * other.w
    }

    pub fn angle(&self, other: &Quat) -> f32 {
        let s = (self.sqr_len() * other.sqr_len()).sqrt();
        (self.dot(other) / s).acos()
    }

    pub fn slerp(&self, other: &Quat, t: f32) -> Quat {
        let theta = self.angle(other);
        if theta.abs() > 0.00001 {
            let d = 1.0 / theta.sin();
            let s0 = ((1.0 - t) * theta).sin();
            let s1 = (t * theta).sin();
            if self.dot(other) < 0.0 {
                return Self {
                    x: (self.x * s0 - other.x * s1) * d,
                    y: (self.y * s0 - other.y * s1) * d,
                    z: (self.z * s0 - other.z * s1) * d,
                    w: (self.w * s0 - other.w * s1) * d,
                };
            } else {
                return Self {
                    x: (self.x * s0 + other.x * s1) * d,
                    y: (self.y * s0 + other.y * s1) * d,
                    z: (self.z * s0 + other.z * s1) * d,
                    w: (self.w * s0 + other.w * s1) * d,
                };
            }
        }
        // Fallback
        *self
    }
}

impl ops::Mul<Self> for Quat {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self {
            x: self.w * b.x + self.x * b.w + self.y * b.z - self.z * b.y,
            y: self.w * b.y + self.y * b.w + self.z * b.x - self.x * b.z,
            z: self.w * b.z + self.z * b.w + self.x * b.y - self.y * b.x,
            w: self.w * b.w - self.x * b.x - self.y * b.y - self.z * b.z,
        }
    }
}
