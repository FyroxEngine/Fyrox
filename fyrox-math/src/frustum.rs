use crate::{aabb::AxisAlignedBoundingBox, plane::Plane};
use nalgebra::Point3;
use nalgebra::{Matrix4, Vector3};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Frustum {
    /// 0 - left, 1 - right, 2 - top, 3 - bottom, 4 - far, 5 - near
    pub planes: [Plane; 6],
    pub corners: [Vector3<f32>; 8],
}

impl Default for Frustum {
    #[inline]
    fn default() -> Self {
        Self::from_view_projection_matrix(Matrix4::new_perspective(
            1.0,
            std::f32::consts::FRAC_PI_2,
            0.01,
            1024.0,
        ))
        .unwrap()
    }
}

impl Frustum {
    pub const LEFT: usize = 0;
    pub const RIGHT: usize = 1;
    pub const TOP: usize = 2;
    pub const BOTTOM: usize = 3;
    pub const FAR: usize = 4;
    pub const NEAR: usize = 5;

    #[inline]
    pub fn from_view_projection_matrix(m: Matrix4<f32>) -> Option<Self> {
        let planes = [
            // Left
            Plane::from_abcd(m[3] + m[0], m[7] + m[4], m[11] + m[8], m[15] + m[12])?,
            // Right
            Plane::from_abcd(m[3] - m[0], m[7] - m[4], m[11] - m[8], m[15] - m[12])?,
            // Top
            Plane::from_abcd(m[3] - m[1], m[7] - m[5], m[11] - m[9], m[15] - m[13])?,
            // Bottom
            Plane::from_abcd(m[3] + m[1], m[7] + m[5], m[11] + m[9], m[15] + m[13])?,
            // Far
            Plane::from_abcd(m[3] - m[2], m[7] - m[6], m[11] - m[10], m[15] - m[14])?,
            // Near
            Plane::from_abcd(m[3] + m[2], m[7] + m[6], m[11] + m[10], m[15] + m[14])?,
        ];

        let corners = [
            planes[Self::LEFT].intersection_point(&planes[Self::TOP], &planes[Self::FAR]),
            planes[Self::LEFT].intersection_point(&planes[Self::BOTTOM], &planes[Self::FAR]),
            planes[Self::RIGHT].intersection_point(&planes[Self::BOTTOM], &planes[Self::FAR]),
            planes[Self::RIGHT].intersection_point(&planes[Self::TOP], &planes[Self::FAR]),
            planes[Self::LEFT].intersection_point(&planes[Self::TOP], &planes[Self::NEAR]),
            planes[Self::LEFT].intersection_point(&planes[Self::BOTTOM], &planes[Self::NEAR]),
            planes[Self::RIGHT].intersection_point(&planes[Self::BOTTOM], &planes[Self::NEAR]),
            planes[Self::RIGHT].intersection_point(&planes[Self::TOP], &planes[Self::NEAR]),
        ];

        Some(Self { planes, corners })
    }

    #[inline]
    pub fn left(&self) -> &Plane {
        self.planes.first().unwrap()
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
        self.corners[0]
    }

    #[inline]
    pub fn left_bottom_front_corner(&self) -> Vector3<f32> {
        self.corners[1]
    }

    #[inline]
    pub fn right_bottom_front_corner(&self) -> Vector3<f32> {
        self.corners[2]
    }

    #[inline]
    pub fn right_top_front_corner(&self) -> Vector3<f32> {
        self.corners[3]
    }

    #[inline]
    pub fn left_top_back_corner(&self) -> Vector3<f32> {
        self.corners[4]
    }

    #[inline]
    pub fn left_bottom_back_corner(&self) -> Vector3<f32> {
        self.corners[5]
    }

    #[inline]
    pub fn right_bottom_back_corner(&self) -> Vector3<f32> {
        self.corners[6]
    }

    #[inline]
    pub fn right_top_back_corner(&self) -> Vector3<f32> {
        self.corners[7]
    }

    #[inline]
    pub fn corners(&self) -> [Vector3<f32>; 8] {
        [
            self.left_top_front_corner(),
            self.left_bottom_front_corner(),
            self.right_bottom_front_corner(),
            self.right_top_front_corner(),
            self.left_top_back_corner(),
            self.left_bottom_back_corner(),
            self.right_bottom_back_corner(),
            self.right_top_back_corner(),
        ]
    }

    #[inline]
    pub fn near_plane_center(&self) -> Vector3<f32> {
        (self.left_top_front_corner()
            + self.left_bottom_front_corner()
            + self.right_bottom_front_corner()
            + self.right_top_front_corner())
        .scale(1.0 / 4.0)
    }

    #[inline]
    pub fn far_plane_center(&self) -> Vector3<f32> {
        (self.left_top_back_corner()
            + self.left_bottom_back_corner()
            + self.right_bottom_back_corner()
            + self.right_top_back_corner())
        .scale(1.0 / 4.0)
    }

    #[inline]
    pub fn view_direction(&self) -> Vector3<f32> {
        self.far_plane_center() - self.near_plane_center()
    }

    #[inline]
    pub fn center(&self) -> Vector3<f32> {
        self.corners()
            .iter()
            .fold(Vector3::default(), |acc, corner| acc + *corner)
            .scale(1.0 / 8.0)
    }

    #[inline]
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

    #[inline]
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

        if self.is_intersects_point_cloud(&corners) {
            return true;
        }

        for corner in self.corners.iter() {
            if aabb.is_contains_point(*corner) {
                return true;
            }
        }

        false
    }

    #[inline]
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

        if self.is_intersects_point_cloud(&corners) {
            return true;
        }

        for corner in self.corners.iter() {
            if aabb.is_contains_point(*corner) {
                return true;
            }
        }

        false
    }

    #[deprecated(
        since = "0.29.0",
        note = "this method does not handle all cases and could give weird results"
    )]
    #[inline]
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

    #[inline]
    pub fn is_contains_point(&self, pt: Vector3<f32>) -> bool {
        for plane in self.planes.iter() {
            if plane.dot(&pt) <= 0.0 {
                return false;
            }
        }
        true
    }

    #[inline]
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

#[cfg(test)]
mod test {
    use crate::aabb::AxisAlignedBoundingBox;
    use crate::{frustum::Frustum, plane::Plane};
    use nalgebra::{Matrix4, Vector3};

    #[test]
    fn test_default_for_frustum() {
        assert_eq!(
            Frustum::default(),
            Frustum::from_view_projection_matrix(Matrix4::new_perspective(
                1.0,
                std::f32::consts::FRAC_PI_2,
                0.01,
                1024.0
            ))
            .unwrap()
        );
    }

    #[test]
    fn test_frustum_from_view_projection_matrix() {
        assert_eq!(
            Frustum::from_view_projection_matrix(Matrix4::new(
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 1.0
            )),
            Some(Frustum {
                planes: [
                    Plane::from_abcd(1.0, 0.0, 0.0, 1.0).unwrap(),
                    Plane::from_abcd(-1.0, 0.0, 0.0, 1.0).unwrap(),
                    Plane::from_abcd(0.0, -1.0, 0.0, 1.0).unwrap(),
                    Plane::from_abcd(0.0, 1.0, 0.0, 1.0).unwrap(),
                    Plane::from_abcd(0.0, 0.0, -1.0, 1.0).unwrap(),
                    Plane::from_abcd(0.0, 0.0, 1.0, 1.0).unwrap(),
                ],
                corners: [
                    Vector3::new(-1.0, 1.0, 1.0),
                    Vector3::new(-1.0, -1.0, 1.0),
                    Vector3::new(1.0, -1.0, 1.0),
                    Vector3::new(1.0, 1.0, 1.0),
                    Vector3::new(-1.0, 1.0, -1.0),
                    Vector3::new(-1.0, -1.0, -1.0),
                    Vector3::new(1.0, -1.0, -1.0),
                    Vector3::new(1.0, 1.0, -1.0),
                ],
            })
        );
    }

    #[test]
    fn test_frustum_planes_and_corners() {
        let f = Frustum::from_view_projection_matrix(Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ))
        .unwrap();

        assert_eq!(f.left(), &Plane::from_abcd(1.0, 0.0, 0.0, 1.0).unwrap());
        assert_eq!(f.right(), &Plane::from_abcd(-1.0, 0.0, 0.0, 1.0).unwrap());
        assert_eq!(f.top(), &Plane::from_abcd(0.0, -1.0, 0.0, 1.0).unwrap());
        assert_eq!(f.bottom(), &Plane::from_abcd(0.0, 1.0, 0.0, 1.0).unwrap());
        assert_eq!(f.far(), &Plane::from_abcd(0.0, 0.0, -1.0, 1.0).unwrap());
        assert_eq!(f.near(), &Plane::from_abcd(0.0, 0.0, 1.0, 1.0).unwrap());

        assert_eq!(
            f.planes(),
            [
                Plane::from_abcd(1.0, 0.0, 0.0, 1.0).unwrap(),
                Plane::from_abcd(-1.0, 0.0, 0.0, 1.0).unwrap(),
                Plane::from_abcd(0.0, -1.0, 0.0, 1.0).unwrap(),
                Plane::from_abcd(0.0, 1.0, 0.0, 1.0).unwrap(),
                Plane::from_abcd(0.0, 0.0, -1.0, 1.0).unwrap(),
                Plane::from_abcd(0.0, 0.0, 1.0, 1.0).unwrap(),
            ]
        );

        assert_eq!(f.left_top_front_corner(), Vector3::new(-1.0, 1.0, 1.0));
        assert_eq!(f.left_bottom_front_corner(), Vector3::new(-1.0, -1.0, 1.0));
        assert_eq!(f.right_bottom_front_corner(), Vector3::new(1.0, -1.0, 1.0));
        assert_eq!(f.right_top_front_corner(), Vector3::new(1.0, 1.0, 1.0));
        assert_eq!(f.left_top_back_corner(), Vector3::new(-1.0, 1.0, -1.0));
        assert_eq!(f.left_bottom_back_corner(), Vector3::new(-1.0, -1.0, -1.0));
        assert_eq!(f.right_bottom_back_corner(), Vector3::new(1.0, -1.0, -1.0));
        assert_eq!(f.right_top_back_corner(), Vector3::new(1.0, 1.0, -1.0));

        assert_eq!(
            f.corners(),
            [
                Vector3::new(-1.0, 1.0, 1.0),
                Vector3::new(-1.0, -1.0, 1.0),
                Vector3::new(1.0, -1.0, 1.0),
                Vector3::new(1.0, 1.0, 1.0),
                Vector3::new(-1.0, 1.0, -1.0),
                Vector3::new(-1.0, -1.0, -1.0),
                Vector3::new(1.0, -1.0, -1.0),
                Vector3::new(1.0, 1.0, -1.0),
            ]
        );
    }

    #[test]
    fn test_frustum_plane_centers() {
        let f = Frustum::from_view_projection_matrix(Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ))
        .unwrap();

        assert_eq!(f.near_plane_center(), Vector3::new(0.0, 0.0, 1.0));
        assert_eq!(f.far_plane_center(), Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(f.view_direction(), Vector3::new(0.0, 0.0, -2.0));
        assert_eq!(f.center(), Vector3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_frustum_is_intersects_point_cloud() {
        let f = Frustum::from_view_projection_matrix(Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ))
        .unwrap();

        assert!(f.is_intersects_point_cloud(&[
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 1.0),
        ]));
        assert!(!f.is_intersects_point_cloud(&[Vector3::new(-1.0, -2.0, 1.0)]));
    }

    #[test]
    fn test_frustum_is_intersects_aabb() {
        let f = Frustum::from_view_projection_matrix(Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ))
        .unwrap();

        assert!(f.is_intersects_aabb(&AxisAlignedBoundingBox::unit()));
        assert!(!f.is_intersects_aabb(&AxisAlignedBoundingBox::from_min_max(
            Vector3::new(5.0, 5.0, 5.0),
            Vector3::new(15.0, 15.0, 15.0)
        )));
    }

    #[test]
    fn test_frustum_is_intersects_aabb_offset() {
        let f = Frustum::from_view_projection_matrix(Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ))
        .unwrap();

        assert!(f.is_intersects_aabb_offset(
            &AxisAlignedBoundingBox::unit(),
            Vector3::new(1.0, 1.0, 1.0)
        ));
        assert!(!f.is_intersects_aabb_offset(
            &AxisAlignedBoundingBox::unit(),
            Vector3::new(10.0, 10.0, 10.0)
        ));
    }

    #[test]
    fn test_frustum_is_contains_point() {
        let f = Frustum::from_view_projection_matrix(Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ))
        .unwrap();

        assert!(f.is_contains_point(Vector3::new(0.0, 0.0, 0.0)));
        assert!(!f.is_contains_point(Vector3::new(10.0, 10.0, 10.0)));
    }

    #[test]
    fn test_frustum_is_intersects_sphere() {
        let f = Frustum::from_view_projection_matrix(Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ))
        .unwrap();

        assert!(f.is_intersects_sphere(Vector3::new(0.0, 0.0, 0.0), 1.0));
        assert!(f.is_intersects_sphere(Vector3::new(0.0, 0.0, 0.0), 2.0));
        assert!(!f.is_intersects_sphere(Vector3::new(10.0, 10.0, 10.0), 1.0));
    }
}
