use crate::algebra::{Matrix4, Vector3};
use crate::{
    math::{aabb::AxisAlignedBoundingBox, plane::Plane},
    visitor::{Visit, VisitResult, Visitor},
};
use nalgebra::Point3;

#[derive(Copy, Clone)]
pub struct Frustum {
    /// 0 - left, 1 - right, 2 - top, 3 - bottom, 4 - far, 5 - near
    planes: [Plane; 6],
}

impl Default for Frustum {
    fn default() -> Self {
        Self::from(Matrix4::new_perspective(
            1.0,
            std::f32::consts::FRAC_PI_2,
            0.01,
            1024.0,
        ))
        .unwrap()
    }
}

impl Frustum {
    pub fn from(m: Matrix4<f32>) -> Option<Self> {
        Some(Self {
            planes: [
                Plane::from_abcd(m[3] + m[0], m[7] + m[4], m[11] + m[8], m[15] + m[12])?,
                Plane::from_abcd(m[3] - m[0], m[7] - m[4], m[11] - m[8], m[15] - m[12])?,
                Plane::from_abcd(m[3] - m[1], m[7] - m[5], m[11] - m[9], m[15] - m[13])?,
                Plane::from_abcd(m[3] + m[1], m[7] + m[5], m[11] + m[9], m[15] + m[13])?,
                Plane::from_abcd(m[3] - m[2], m[7] - m[6], m[11] - m[10], m[15] - m[14])?,
                Plane::from_abcd(m[3] + m[2], m[7] + m[6], m[11] + m[10], m[15] + m[14])?,
            ],
        })
    }

    #[inline]
    pub fn left(&self) -> &Plane {
        self.planes.get(0).unwrap()
    }

    #[inline]
    pub fn right(&self) -> &Plane {
        self.planes.get(1).unwrap()
    }

    #[inline]
    pub fn top(&self) -> &Plane {
        self.planes.get(2).unwrap()
    }

    #[inline]
    pub fn bottom(&self) -> &Plane {
        self.planes.get(3).unwrap()
    }

    #[inline]
    pub fn far(&self) -> &Plane {
        self.planes.get(4).unwrap()
    }

    #[inline]
    pub fn near(&self) -> &Plane {
        self.planes.get(5).unwrap()
    }

    #[inline]
    pub fn planes(&self) -> &[Plane] {
        &self.planes
    }

    #[inline]
    pub fn left_top_front_corner(&self) -> Vector3<f32> {
        self.left().intersection_point(self.top(), self.far())
    }

    #[inline]
    pub fn left_bottom_front_corner(&self) -> Vector3<f32> {
        self.left().intersection_point(self.bottom(), self.far())
    }

    #[inline]
    pub fn right_bottom_front_corner(&self) -> Vector3<f32> {
        self.right().intersection_point(self.bottom(), self.far())
    }

    #[inline]
    pub fn right_top_front_corner(&self) -> Vector3<f32> {
        self.right().intersection_point(self.top(), self.far())
    }

    #[inline]
    pub fn left_top_back_corner(&self) -> Vector3<f32> {
        self.left().intersection_point(self.top(), self.near())
    }

    #[inline]
    pub fn left_bottom_back_corner(&self) -> Vector3<f32> {
        self.left().intersection_point(self.bottom(), self.near())
    }

    #[inline]
    pub fn right_bottom_back_corner(&self) -> Vector3<f32> {
        self.right().intersection_point(self.bottom(), self.near())
    }

    #[inline]
    pub fn right_top_back_corner(&self) -> Vector3<f32> {
        self.right().intersection_point(self.top(), self.near())
    }

    pub fn is_intersects_point_cloud(&self, points: &[Vector3<f32>]) -> bool {
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
            Vector3::new(aabb.min.x, aabb.min.y, aabb.min.z),
            Vector3::new(aabb.min.x, aabb.min.y, aabb.max.z),
            Vector3::new(aabb.max.x, aabb.min.y, aabb.max.z),
            Vector3::new(aabb.max.x, aabb.min.y, aabb.min.z),
            Vector3::new(aabb.min.x, aabb.max.y, aabb.min.z),
            Vector3::new(aabb.min.x, aabb.max.y, aabb.max.z),
            Vector3::new(aabb.max.x, aabb.max.y, aabb.max.z),
            Vector3::new(aabb.max.x, aabb.max.y, aabb.min.z),
        ];

        self.is_intersects_point_cloud(&corners)
    }

    pub fn is_intersects_aabb_offset(
        &self,
        aabb: &AxisAlignedBoundingBox,
        offset: Vector3<f32>,
    ) -> bool {
        let corners = [
            Vector3::new(aabb.min.x, aabb.min.y, aabb.min.z) + offset,
            Vector3::new(aabb.min.x, aabb.min.y, aabb.max.z) + offset,
            Vector3::new(aabb.max.x, aabb.min.y, aabb.max.z) + offset,
            Vector3::new(aabb.max.x, aabb.min.y, aabb.min.z) + offset,
            Vector3::new(aabb.min.x, aabb.max.y, aabb.min.z) + offset,
            Vector3::new(aabb.min.x, aabb.max.y, aabb.max.z) + offset,
            Vector3::new(aabb.max.x, aabb.max.y, aabb.max.z) + offset,
            Vector3::new(aabb.max.x, aabb.max.y, aabb.min.z) + offset,
        ];

        self.is_intersects_point_cloud(&corners)
    }

    pub fn is_intersects_aabb_transform(
        &self,
        aabb: &AxisAlignedBoundingBox,
        transform: &Matrix4<f32>,
    ) -> bool {
        if self.is_contains_point(
            transform
                .transform_point(&Point3::from(aabb.center()))
                .coords,
        ) {
            return true;
        }

        let corners = [
            transform
                .transform_point(&Point3::new(aabb.min.x, aabb.min.y, aabb.min.z))
                .coords,
            transform
                .transform_point(&Point3::new(aabb.min.x, aabb.min.y, aabb.max.z))
                .coords,
            transform
                .transform_point(&Point3::new(aabb.max.x, aabb.min.y, aabb.max.z))
                .coords,
            transform
                .transform_point(&Point3::new(aabb.max.x, aabb.min.y, aabb.min.z))
                .coords,
            transform
                .transform_point(&Point3::new(aabb.min.x, aabb.max.y, aabb.min.z))
                .coords,
            transform
                .transform_point(&Point3::new(aabb.min.x, aabb.max.y, aabb.max.z))
                .coords,
            transform
                .transform_point(&Point3::new(aabb.max.x, aabb.max.y, aabb.max.z))
                .coords,
            transform
                .transform_point(&Point3::new(aabb.max.x, aabb.max.y, aabb.min.z))
                .coords,
        ];

        self.is_intersects_point_cloud(&corners)
    }

    pub fn is_contains_point(&self, pt: Vector3<f32>) -> bool {
        for plane in self.planes.iter() {
            if plane.dot(&pt) <= 0.0 {
                return false;
            }
        }
        true
    }

    pub fn is_intersects_sphere(&self, p: Vector3<f32>, r: f32) -> bool {
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
