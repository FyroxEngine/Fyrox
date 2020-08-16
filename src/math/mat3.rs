use crate::math::quat::Quat;
use crate::math::vec3::Vec3;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Mat3 {
    pub f: [f32; 9],
}

impl Mat3 {
    pub fn identity() -> Self {
        Self {
            f: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        }
    }

    pub fn from_vectors(xaxis: Vec3, yaxis: Vec3, zaxis: Vec3) -> Self {
        let xaxis = xaxis
            .normalized()
            .unwrap_or_else(|| Vec3::new(1.0, 0.0, 0.0));
        let yaxis = yaxis
            .normalized()
            .unwrap_or_else(|| Vec3::new(0.0, 1.0, 0.0));
        let zaxis = zaxis
            .normalized()
            .unwrap_or_else(|| Vec3::new(0.0, 0.0, 1.0));
        Self {
            f: [
                xaxis.x, yaxis.x, zaxis.x, xaxis.y, yaxis.y, zaxis.y, xaxis.z, yaxis.z, zaxis.z,
            ],
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
                [
                    1.0 - (yy + zz),
                    xy + wz,
                    xz - wy,
                    xy - wz,
                    1.0 - (xx + zz),
                    yz + wx,
                    xz + wy,
                    yz - wx,
                    1.0 - (xx + yy),
                ]
            },
        }
    }

    pub fn transform_vector(&self, v: Vec3) -> Vec3 {
        Vec3 {
            x: v.x * self.f[0] + v.y * self.f[3] + v.z * self.f[6],
            y: v.x * self.f[1] + v.y * self.f[4] + v.z * self.f[7],
            z: v.x * self.f[2] + v.y * self.f[5] + v.z * self.f[8],
        }
    }

    /// Returns "side" vector from basis. (points right)
    pub fn side(&self) -> Vec3 {
        Vec3::new(self.f[0], self.f[1], self.f[2])
    }

    /// Returns "up" vector from basis.
    pub fn up(&self) -> Vec3 {
        Vec3::new(self.f[3], self.f[4], self.f[5])
    }

    /// Returns "look" vector from basis. (points into screen)
    pub fn look(&self) -> Vec3 {
        Vec3::new(self.f[6], self.f[7], self.f[8])
    }
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::identity()
    }
}
