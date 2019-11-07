use crate::{
    math::{
        mat4::Mat4,
        plane::Plane,
        vec3::Vec3,
        aabb::AxisAlignedBoundingBox,
    },
    visitor::{Visit, Visitor, VisitResult}
};

#[derive(Copy, Clone)]
pub struct Frustum {
    planes: [Plane; 6]
}

impl Default for Frustum {
    fn default() -> Self {
        Self::from(Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.01, 1024.0)).unwrap()
    }
}

impl Frustum {
    pub fn from(m: Mat4) -> Result<Self, ()> {
        Ok(Self {
            planes: [
                Plane::from_abcd(m.f[3] + m.f[0], m.f[7] + m.f[4], m.f[11] + m.f[8], m.f[15] + m.f[12])?,
                Plane::from_abcd(m.f[3] - m.f[0], m.f[7] - m.f[4], m.f[11] - m.f[8], m.f[15] - m.f[12])?,
                Plane::from_abcd(m.f[3] - m.f[1], m.f[7] - m.f[5], m.f[11] - m.f[9], m.f[15] - m.f[13])?,
                Plane::from_abcd(m.f[3] + m.f[1], m.f[7] + m.f[5], m.f[11] + m.f[9], m.f[15] + m.f[13])?,
                Plane::from_abcd(m.f[3] - m.f[2], m.f[7] - m.f[6], m.f[11] - m.f[10], m.f[15] - m.f[14])?,
                Plane::from_abcd(m.f[3] + m.f[2], m.f[7] + m.f[6], m.f[11] + m.f[10], m.f[15] + m.f[14])?,
            ]
        })
    }

    pub fn is_intersects_point_cloud(&self, points: &[Vec3]) -> bool {
        for plane in self.planes.iter() {
            let mut back_points = 0;
            for point in points {
                if plane.dot(point) <= 0.0 {
                    back_points += 1;
                    if back_points >= points.len() {
                        // All points are behind current plane.
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn is_intersects_aabb(&self, aabb: &AxisAlignedBoundingBox) -> bool {
        let corners = [
            Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z),
            Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z),
            Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z),
            Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z),
            Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z),
            Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z),
            Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z),
            Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z),
        ];

        self.is_intersects_point_cloud(&corners)
    }

    pub fn is_intersects_aabb_offset(&self, aabb: &AxisAlignedBoundingBox, offset: Vec3) -> bool {
        let corners = [
            Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z) + offset,
            Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z) + offset,
            Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z) + offset,
            Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z) + offset,
            Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z) + offset,
            Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z) + offset,
            Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z) + offset,
            Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z) + offset,
        ];

        self.is_intersects_point_cloud(&corners)
    }

    pub fn is_intersects_aabb_transform(&self, aabb: &AxisAlignedBoundingBox, transform: &Mat4) -> bool {
        let corners = [
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.min.y, aabb.min.z)),
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.min.y, aabb.max.z)),
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.min.y, aabb.max.z)),
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.min.y, aabb.min.z)),
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.max.y, aabb.min.z)),
            transform.transform_vector(Vec3::new(aabb.min.x, aabb.max.y, aabb.max.z)),
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.max.y, aabb.max.z)),
            transform.transform_vector(Vec3::new(aabb.max.x, aabb.max.y, aabb.min.z)),
        ];

        self.is_intersects_point_cloud(&corners)
    }

    pub fn is_contains_point(&self, pt: Vec3) -> bool {
        for plane in self.planes.iter() {
            if plane.dot(&pt) <= 0.0 {
                return false;
            }
        }
        true
    }

    pub fn is_intersects_sphere(&self, p: Vec3, r: f32) -> bool {
        for plane in self.planes.iter() {
            let d = plane.dot(&p);
            if d < -r {
                return false;
            }
            if d.abs() < r {
                return true;
            }
        }
        true
    }
}

impl Visit for Frustum {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.planes[0].visit("Left", visitor)?;
        self.planes[1].visit("Right", visitor)?;
        self.planes[2].visit("Top", visitor)?;
        self.planes[3].visit("Bottom", visitor)?;
        self.planes[4].visit("Far", visitor)?;
        self.planes[5].visit("Near", visitor)?;

        visitor.leave_region()
    }
}