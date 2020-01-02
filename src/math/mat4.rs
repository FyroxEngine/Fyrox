use std::ops;
use crate::{
    math::{
        vec3::*,
        quat::*,
    }
};

#[derive(Copy, Clone, Debug)]
pub struct Mat4 {
    pub f: [f32; 16]
}

impl Default for Mat4 {
    fn default() -> Self {
        Mat4::IDENTITY
    }
}

impl Mat4 {
    pub const IDENTITY: Self = Self {
        f: [1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0]
    };

    pub fn scale(v: Vec3) -> Self {
        Self {
            f: [v.x, 0.0, 0.0, 0.0,
                0.0, v.y, 0.0, 0.0,
                0.0, 0.0, v.z, 0.0,
                0.0, 0.0, 0.0, 1.0]
        }
    }

    pub fn translate(v: Vec3) -> Self {
        Self {
            f: [1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                v.x, v.y, v.z, 1.0]
        }
    }

    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, z_near: f32, z_far: f32) -> Self {
        Self {
            f: [
                2.0 / (right - left), 0.0, 0.0, 0.0,
                0.0, 2.0 / (top - bottom), 0.0, 0.0,
                0.0, 0.0, 1.0 / (z_far - z_near), 0.0,
                (left + right) / (left - right), (top + bottom) / (bottom - top),
                z_near / (z_near - z_far), 1.0]
        }
    }

    pub fn perspective(fov_rad: f32, aspect: f32, z_near: f32, z_far: f32) -> Self {
        let y_scale = 1.0 / (fov_rad * 0.5).tan();
        let x_scale = y_scale / aspect;

        Self {
            f: [
                x_scale, 0.0, 0.0, 0.0,
                0.0, y_scale, 0.0, 0.0,
                0.0, 0.0, z_far / (z_near - z_far), -1.0,
                0.0, 0.0, z_near * z_far / (z_near - z_far), 0.0
            ]
        }
    }

    pub fn from_quat(q: Quat) -> Self {
        let s = 2.0 / q.sqr_len();

        let xs = q.x * s;
        let ys = q.y * s;
        let zs = q.z * s;

        let wx = q.w * xs;
        let wy = q.w * ys;
        let wz = q.w * zs;

        let xx = q.x * xs;
        let xy = q.x * ys;
        let xz = q.x * zs;

        let yy = q.y * ys;
        let yz = q.y * zs;
        let zz = q.z * zs;

        Self {
            f: {
                [1.0 - (yy + zz), xy + wz, xz - wy, 0.0,
                    xy - wz, 1.0 - (xx + zz), yz + wx, 0.0,
                    xz + wy, yz - wx, 1.0 - (xx + yy), 0.0,
                    0.0, 0.0, 0.0, 1.0]
            }
        }
    }

    pub fn look_at(eye: Vec3, at: Vec3, up: Vec3) -> Option<Mat4> {
        let zaxis = (eye - at).normalized()?;
        let xaxis = up.cross(&zaxis).normalized()?;
        let yaxis = zaxis.cross(&xaxis).normalized()?;

        Some(Self {
            f: [
                xaxis.x, yaxis.x, zaxis.x, 0.0,
                xaxis.y, yaxis.y, zaxis.y, 0.0,
                xaxis.z, yaxis.z, zaxis.z, 0.0,
                -xaxis.dot(&eye), -yaxis.dot(&eye), -zaxis.dot(&eye), 1.0,
            ]
        })
    }

    /// Returns Ok(inverted_matrix) in case if matrix is invertible,
    /// or Err(()) if matrix has determinant == 0 and so not invertible.
    pub fn inverse(&self) -> Result<Mat4, ()> {
        let f = &self.f;
        let mut temp = Mat4 {
            f: [
                f[5] * f[10] * f[15] - f[5] * f[14] * f[11] - f[6] * f[9] * f[15] + f[6] * f[13] * f[11] + f[7] * f[9] * f[14] - f[7] * f[13] * f[10],
                -f[1] * f[10] * f[15] + f[1] * f[14] * f[11] + f[2] * f[9] * f[15] - f[2] * f[13] * f[11] - f[3] * f[9] * f[14] + f[3] * f[13] * f[10],
                f[1] * f[6] * f[15] - f[1] * f[14] * f[7] - f[2] * f[5] * f[15] + f[2] * f[13] * f[7] + f[3] * f[5] * f[14] - f[3] * f[13] * f[6],
                -f[1] * f[6] * f[11] + f[1] * f[10] * f[7] + f[2] * f[5] * f[11] - f[2] * f[9] * f[7] - f[3] * f[5] * f[10] + f[3] * f[9] * f[6],
                -f[4] * f[10] * f[15] + f[4] * f[14] * f[11] + f[6] * f[8] * f[15] - f[6] * f[12] * f[11] - f[7] * f[8] * f[14] + f[7] * f[12] * f[10],
                f[0] * f[10] * f[15] - f[0] * f[14] * f[11] - f[2] * f[8] * f[15] + f[2] * f[12] * f[11] + f[3] * f[8] * f[14] - f[3] * f[12] * f[10],
                -f[0] * f[6] * f[15] + f[0] * f[14] * f[7] + f[2] * f[4] * f[15] - f[2] * f[12] * f[7] - f[3] * f[4] * f[14] + f[3] * f[12] * f[6],
                f[0] * f[6] * f[11] - f[0] * f[10] * f[7] - f[2] * f[4] * f[11] + f[2] * f[8] * f[7] + f[3] * f[4] * f[10] - f[3] * f[8] * f[6],
                f[4] * f[9] * f[15] - f[4] * f[13] * f[11] - f[5] * f[8] * f[15] + f[5] * f[12] * f[11] + f[7] * f[8] * f[13] - f[7] * f[12] * f[9],
                -f[0] * f[9] * f[15] + f[0] * f[13] * f[11] + f[1] * f[8] * f[15] - f[1] * f[12] * f[11] - f[3] * f[8] * f[13] + f[3] * f[12] * f[9],
                f[0] * f[5] * f[15] - f[0] * f[13] * f[7] - f[1] * f[4] * f[15] + f[1] * f[12] * f[7] + f[3] * f[4] * f[13] - f[3] * f[12] * f[5],
                -f[0] * f[5] * f[11] + f[0] * f[9] * f[7] + f[1] * f[4] * f[11] - f[1] * f[8] * f[7] - f[3] * f[4] * f[9] + f[3] * f[8] * f[5],
                -f[4] * f[9] * f[14] + f[4] * f[13] * f[10] + f[5] * f[8] * f[14] - f[5] * f[12] * f[10] - f[6] * f[8] * f[13] + f[6] * f[12] * f[9],
                f[0] * f[9] * f[14] - f[0] * f[13] * f[10] - f[1] * f[8] * f[14] + f[1] * f[12] * f[10] + f[2] * f[8] * f[13] - f[2] * f[12] * f[9],
                -f[0] * f[5] * f[14] + f[0] * f[13] * f[6] + f[1] * f[4] * f[14] - f[1] * f[12] * f[6] - f[2] * f[4] * f[13] + f[2] * f[12] * f[5],
                f[0] * f[5] * f[10] - f[0] * f[9] * f[6] - f[1] * f[4] * f[10] + f[1] * f[8] * f[6] + f[2] * f[4] * f[9] - f[2] * f[8] * f[5],
            ]
        };

        let mut det = f[0] * temp.f[0] + f[4] * temp.f[1] + f[8] * temp.f[2] + f[12] * temp.f[3];
        if det.abs() >= std::f32::EPSILON {
            det = 1.0 / det;
            for i in 0..16 {
                temp.f[i] *= det;
            }
            return Ok(temp);
        }

        Err(())
    }

    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        Vec3 {
            x: v.x * self.f[0] + v.y * self.f[4] + v.z * self.f[8] + self.f[12],
            y: v.x * self.f[1] + v.y * self.f[5] + v.z * self.f[9] + self.f[13],
            z: v.x * self.f[2] + v.y * self.f[6] + v.z * self.f[10] + self.f[14],
        }
    }

    pub fn transform_vector_normal(&self, v: Vec3) -> Vec3 {
        Vec3 {
            x: v.x * self.f[0] + v.y * self.f[4] + v.z * self.f[8],
            y: v.x * self.f[1] + v.y * self.f[5] + v.z * self.f[9],
            z: v.x * self.f[2] + v.y * self.f[6] + v.z * self.f[10],
        }
    }

    /// Returns "side" vector from basis. (points right)
    pub fn side(&self) -> Vec3 {
        Vec3::new(self.f[0], self.f[1], self.f[2])
    }

    /// Returns "up" vector from basis.
    pub fn up(&self) -> Vec3 {
        Vec3::new(self.f[4], self.f[5], self.f[6])
    }

    /// Returns "look" vector from basis. (points into screen)
    pub fn look(&self) -> Vec3 {
        Vec3::new(self.f[8], self.f[9], self.f[10])
    }

    /// Returns translation part of matrix.
    pub fn position(&self) -> Vec3 {
        Vec3::new(self.f[12], self.f[13], self.f[14])
    }
}

impl ops::Mul<Self> for Mat4 {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self {
            f: [
                self.f[0] * b.f[0] + self.f[4] * b.f[1] + self.f[8] * b.f[2] + self.f[12] * b.f[3],
                self.f[1] * b.f[0] + self.f[5] * b.f[1] + self.f[9] * b.f[2] + self.f[13] * b.f[3],
                self.f[2] * b.f[0] + self.f[6] * b.f[1] + self.f[10] * b.f[2] + self.f[14] * b.f[3],
                self.f[3] * b.f[0] + self.f[7] * b.f[1] + self.f[11] * b.f[2] + self.f[15] * b.f[3],
                self.f[0] * b.f[4] + self.f[4] * b.f[5] + self.f[8] * b.f[6] + self.f[12] * b.f[7],
                self.f[1] * b.f[4] + self.f[5] * b.f[5] + self.f[9] * b.f[6] + self.f[13] * b.f[7],
                self.f[2] * b.f[4] + self.f[6] * b.f[5] + self.f[10] * b.f[6] + self.f[14] * b.f[7],
                self.f[3] * b.f[4] + self.f[7] * b.f[5] + self.f[11] * b.f[6] + self.f[15] * b.f[7],
                self.f[0] * b.f[8] + self.f[4] * b.f[9] + self.f[8] * b.f[10] + self.f[12] * b.f[11],
                self.f[1] * b.f[8] + self.f[5] * b.f[9] + self.f[9] * b.f[10] + self.f[13] * b.f[11],
                self.f[2] * b.f[8] + self.f[6] * b.f[9] + self.f[10] * b.f[10] + self.f[14] * b.f[11],
                self.f[3] * b.f[8] + self.f[7] * b.f[9] + self.f[11] * b.f[10] + self.f[15] * b.f[11],
                self.f[0] * b.f[12] + self.f[4] * b.f[13] + self.f[8] * b.f[14] + self.f[12] * b.f[15],
                self.f[1] * b.f[12] + self.f[5] * b.f[13] + self.f[9] * b.f[14] + self.f[13] * b.f[15],
                self.f[2] * b.f[12] + self.f[6] * b.f[13] + self.f[10] * b.f[14] + self.f[14] * b.f[15],
                self.f[3] * b.f[12] + self.f[7] * b.f[13] + self.f[11] * b.f[14] + self.f[15] * b.f[15]
            ]
        }
    }
}


