use std::ops;
use crate::math::vec3::*;
use crate::math::quat::*;

#[derive(Copy, Clone)]
pub struct Mat4 {
    pub f: [f32; 16]
}

impl Mat4 {
    pub fn identity() -> Self {
        Self {
            f: [1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0]
        }
    }

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
                2.0 / (right - left),
                0.0,
                0.0,
                0.0,
                0.0,
                2.0 / (top - bottom),
                0.0,
                0.0,
                0.0,
                0.0,
                1.0 / (z_far - z_near),
                0.0,
                (left + right) / (left - right),
                (top + bottom) / (bottom - top),
                z_near / (z_near - z_far),
                1.0]
        }
    }

    pub fn perspective(fov_rad: f32, aspect: f32, z_near: f32, z_far: f32) -> Self {
        let y_scale = 1.0 / (fov_rad * 0.5).tan();
        let x_scale = y_scale / aspect;

        Self {
            f: [
                x_scale,
                0.0,
                0.0,
                0.0,
                0.0,
                y_scale,
                0.0,
                0.0,
                0.0,
                0.0,
                z_far / (z_near - z_far),
                -1.0,
                0.0,
                0.0,
                z_near * z_far / (z_near - z_far),
                0.0
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

    pub fn mul(a: Self, b: Self) -> Self {
        Self {
            f: [
                a.f[0] * b.f[0] + a.f[4] * b.f[1] + a.f[8] * b.f[2] + a.f[12] * b.f[3],
                a.f[1] * b.f[0] + a.f[5] * b.f[1] + a.f[9] * b.f[2] + a.f[13] * b.f[3],
                a.f[2] * b.f[0] + a.f[6] * b.f[1] + a.f[10] * b.f[2] + a.f[14] * b.f[3],
                a.f[3] * b.f[0] + a.f[7] * b.f[1] + a.f[11] * b.f[2] + a.f[15] * b.f[3],
                a.f[0] * b.f[4] + a.f[4] * b.f[5] + a.f[8] * b.f[6] + a.f[12] * b.f[7],
                a.f[1] * b.f[4] + a.f[5] * b.f[5] + a.f[9] * b.f[6] + a.f[13] * b.f[7],
                a.f[2] * b.f[4] + a.f[6] * b.f[5] + a.f[10] * b.f[6] + a.f[14] * b.f[7],
                a.f[3] * b.f[4] + a.f[7] * b.f[5] + a.f[11] * b.f[6] + a.f[15] * b.f[7],
                a.f[0] * b.f[8] + a.f[4] * b.f[9] + a.f[8] * b.f[10] + a.f[12] * b.f[11],
                a.f[1] * b.f[8] + a.f[5] * b.f[9] + a.f[9] * b.f[10] + a.f[13] * b.f[11],
                a.f[2] * b.f[8] + a.f[6] * b.f[9] + a.f[10] * b.f[10] + a.f[14] * b.f[11],
                a.f[3] * b.f[8] + a.f[7] * b.f[9] + a.f[11] * b.f[10] + a.f[15] * b.f[11],
                a.f[0] * b.f[12] + a.f[4] * b.f[13] + a.f[8] * b.f[14] + a.f[12] * b.f[15],
                a.f[1] * b.f[12] + a.f[5] * b.f[13] + a.f[9] * b.f[14] + a.f[13] * b.f[15],
                a.f[2] * b.f[12] + a.f[6] * b.f[13] + a.f[10] * b.f[14] + a.f[14] * b.f[15],
                a.f[3] * b.f[12] + a.f[7] * b.f[13] + a.f[11] * b.f[14] + a.f[15] * b.f[15]
            ]
        }
    }

    pub fn look_at(eye: Vec3, at: Vec3, up: Vec3) -> Result<Mat4, &'static str> {
        let zaxis = (at - eye).normalized()?;
        let xaxis = up.cross(&zaxis).normalized()?;
        let yaxis = zaxis.cross(&xaxis).normalized()?;

        Ok(Self {
            f: [
                xaxis.x,
                yaxis.x,
                zaxis.x,
                0.0,

                xaxis.y,
                yaxis.y,
                zaxis.y,
                0.0,

                xaxis.z,
                yaxis.z,
                zaxis.z,
                0.0,

                -xaxis.dot(&eye),
                -yaxis.dot(&eye),
                -zaxis.dot(&eye), // ????????????
                1.0,
            ]
        })
    }

    pub fn inverse(&self) -> Result<Mat4, &str> {
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
        if det.abs() > 0.000001 {
            det = 1.0 / det;
            for i in 0..16 {
                temp.f[i] = temp.f[i] * det;
            }
            return Ok(temp);
        }
        Err("matrix is not invertible, determinant == 0")
    }
}

impl ops::Mul<Self> for Mat4 {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Mat4::mul(self, b)
    }
}