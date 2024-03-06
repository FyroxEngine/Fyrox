// Clippy complains about normal mathematical symbols like A, B, C for quadratic equation.
#![allow(clippy::many_single_char_names)]

use crate::aabb::AxisAlignedBoundingBox;
use crate::{is_point_inside_triangle, plane::Plane, solve_quadratic};
use nalgebra::{Matrix4, Point3, Vector3};

#[derive(Copy, Clone, Debug)]
pub struct Ray {
    pub origin: Vector3<f32>,
    pub dir: Vector3<f32>,
}

impl Default for Ray {
    #[inline]
    fn default() -> Self {
        Ray {
            origin: Vector3::new(0.0, 0.0, 0.0),
            dir: Vector3::new(0.0, 0.0, 1.0),
        }
    }
}

/// Pair of ray equation parameters.
#[derive(Clone, Debug, Copy)]
pub struct IntersectionResult {
    pub min: f32,
    pub max: f32,
}

impl IntersectionResult {
    #[inline]
    pub fn from_slice(roots: &[f32]) -> Self {
        let mut min = f32::MAX;
        let mut max = -f32::MAX;
        for n in roots {
            min = min.min(*n);
            max = max.max(*n);
        }
        Self { min, max }
    }

    #[inline]
    pub fn from_set(results: &[Option<IntersectionResult>]) -> Option<Self> {
        let mut result = None;
        for v in results {
            match result {
                None => result = *v,
                Some(ref mut result) => {
                    if let Some(v) = v {
                        result.merge(v.min);
                        result.merge(v.max);
                    }
                }
            }
        }
        result
    }

    /// Updates min and max ray equation parameters according to a new parameter -
    /// expands range if `param` was outside of that range.
    #[inline]
    pub fn merge(&mut self, param: f32) {
        if param < self.min {
            self.min = param;
        }
        if param > self.max {
            self.max = param;
        }
    }

    #[inline]
    pub fn merge_slice(&mut self, params: &[f32]) {
        for param in params {
            self.merge(*param)
        }
    }
}

pub enum CylinderKind {
    Infinite,
    Finite,
    Capped,
}

impl Ray {
    /// Creates ray from two points. May fail if begin == end.
    #[inline]
    pub fn from_two_points(begin: Vector3<f32>, end: Vector3<f32>) -> Self {
        Ray {
            origin: begin,
            dir: end - begin,
        }
    }

    #[inline]
    pub fn new(origin: Vector3<f32>, dir: Vector3<f32>) -> Self {
        Self { origin, dir }
    }

    /// Checks intersection with sphere. Returns two intersection points or none
    /// if there was no intersection.
    #[inline]
    pub fn sphere_intersection_points(
        &self,
        position: &Vector3<f32>,
        radius: f32,
    ) -> Option<[Vector3<f32>; 2]> {
        self.try_eval_points(self.sphere_intersection(position, radius))
    }

    #[inline]
    pub fn sphere_intersection(
        &self,
        position: &Vector3<f32>,
        radius: f32,
    ) -> Option<IntersectionResult> {
        let d = self.origin - *position;
        let a = self.dir.dot(&self.dir);
        let b = 2.0 * self.dir.dot(&d);
        let c = d.dot(&d) - radius * radius;
        solve_quadratic(a, b, c).map(|roots| IntersectionResult::from_slice(&roots))
    }

    /// Checks intersection with sphere.
    #[inline]
    pub fn is_intersect_sphere(&self, position: &Vector3<f32>, radius: f32) -> bool {
        let d = self.origin - position;
        let a = self.dir.dot(&self.dir);
        let b = 2.0 * self.dir.dot(&d);
        let c = d.dot(&d) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;
        discriminant >= 0.0
    }

    /// Returns t factor (at pt=o+d*t equation) for projection of given point at ray
    #[inline]
    pub fn project_point(&self, point: &Vector3<f32>) -> f32 {
        (point - self.origin).dot(&self.dir) / self.dir.norm_squared()
    }

    /// Returns point on ray which defined by pt=o+d*t equation.
    #[inline]
    pub fn get_point(&self, t: f32) -> Vector3<f32> {
        self.origin + self.dir.scale(t)
    }

    #[inline]
    pub fn box_intersection(
        &self,
        min: &Vector3<f32>,
        max: &Vector3<f32>,
    ) -> Option<IntersectionResult> {
        let (mut tmin, mut tmax) = if self.dir.x >= 0.0 {
            (
                (min.x - self.origin.x) / self.dir.x,
                (max.x - self.origin.x) / self.dir.x,
            )
        } else {
            (
                (max.x - self.origin.x) / self.dir.x,
                (min.x - self.origin.x) / self.dir.x,
            )
        };

        let (tymin, tymax) = if self.dir.y >= 0.0 {
            (
                (min.y - self.origin.y) / self.dir.y,
                (max.y - self.origin.y) / self.dir.y,
            )
        } else {
            (
                (max.y - self.origin.y) / self.dir.y,
                (min.y - self.origin.y) / self.dir.y,
            )
        };

        if tmin > tymax || (tymin > tmax) {
            return None;
        }
        if tymin > tmin {
            tmin = tymin;
        }
        if tymax < tmax {
            tmax = tymax;
        }
        let (tzmin, tzmax) = if self.dir.z >= 0.0 {
            (
                (min.z - self.origin.z) / self.dir.z,
                (max.z - self.origin.z) / self.dir.z,
            )
        } else {
            (
                (max.z - self.origin.z) / self.dir.z,
                (min.z - self.origin.z) / self.dir.z,
            )
        };

        if (tmin > tzmax) || (tzmin > tmax) {
            return None;
        }
        if tzmin > tmin {
            tmin = tzmin;
        }
        if tzmax < tmax {
            tmax = tzmax;
        }
        if tmin <= 1.0 && tmax >= 0.0 {
            Some(IntersectionResult {
                min: tmin,
                max: tmax,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn box_intersection_points(
        &self,
        min: &Vector3<f32>,
        max: &Vector3<f32>,
    ) -> Option<[Vector3<f32>; 2]> {
        self.try_eval_points(self.box_intersection(min, max))
    }

    #[inline]
    pub fn aabb_intersection(&self, aabb: &AxisAlignedBoundingBox) -> Option<IntersectionResult> {
        self.box_intersection(&aabb.min, &aabb.max)
    }

    #[inline]
    pub fn aabb_intersection_points(
        &self,
        aabb: &AxisAlignedBoundingBox,
    ) -> Option<[Vector3<f32>; 2]> {
        self.box_intersection_points(&aabb.min, &aabb.max)
    }

    /// Solves plane equation in order to find ray equation parameter.
    /// There is no intersection if result < 0.
    #[inline]
    pub fn plane_intersection(&self, plane: &Plane) -> f32 {
        let u = -(self.origin.dot(&plane.normal) + plane.d);
        let v = self.dir.dot(&plane.normal);
        u / v
    }

    #[inline]
    pub fn plane_intersection_point(&self, plane: &Plane) -> Option<Vector3<f32>> {
        let t = self.plane_intersection(plane);
        if !(0.0..=1.0).contains(&t) {
            None
        } else {
            Some(self.get_point(t))
        }
    }

    #[inline]
    pub fn triangle_intersection(
        &self,
        vertices: &[Vector3<f32>; 3],
    ) -> Option<(f32, Vector3<f32>)> {
        let ba = vertices[1] - vertices[0];
        let ca = vertices[2] - vertices[0];
        let plane = Plane::from_normal_and_point(&ba.cross(&ca), &vertices[0])?;

        let t = self.plane_intersection(&plane);
        if (0.0..=1.0).contains(&t) {
            let point = self.get_point(t);
            if is_point_inside_triangle(&point, vertices) {
                return Some((t, point));
            }
        }
        None
    }

    #[inline]
    pub fn triangle_intersection_point(
        &self,
        vertices: &[Vector3<f32>; 3],
    ) -> Option<Vector3<f32>> {
        let ba = vertices[1] - vertices[0];
        let ca = vertices[2] - vertices[0];
        let plane = Plane::from_normal_and_point(&ba.cross(&ca), &vertices[0])?;

        if let Some(point) = self.plane_intersection_point(&plane) {
            if is_point_inside_triangle(&point, vertices) {
                return Some(point);
            }
        }
        None
    }

    /// Generic ray-cylinder intersection test.
    ///
    /// <https://mrl.nyu.edu/~dzorin/rend05/lecture2.pdf>
    ///
    ///  Infinite cylinder oriented along line pa + va * t:
    ///      sqr_len(q - pa - dot(va, q - pa) * va) - r ^ 2 = 0
    ///  where q - point on cylinder, substitute q with ray p + v * t:
    ///     sqr_len(p - pa + vt - dot(va, p - pa + vt) * va) - r ^ 2 = 0
    ///  reduce to A * t * t + B * t + C = 0 (quadratic equation), where:
    ///     A = sqr_len(v - dot(v, va) * va)
    ///     B = 2 * dot(v - dot(v, va) * va, dp - dot(dp, va) * va)
    ///     C = sqr_len(dp - dot(dp, va) * va) - r ^ 2
    ///     where dp = p - pa
    ///  to find intersection points we have to solve quadratic equation
    ///  to get root which will be t parameter of ray equation.
    #[inline]
    pub fn cylinder_intersection(
        &self,
        pa: &Vector3<f32>,
        pb: &Vector3<f32>,
        r: f32,
        kind: CylinderKind,
    ) -> Option<IntersectionResult> {
        let va = (*pb - *pa)
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(|| Vector3::new(0.0, 1.0, 0.0));
        let vl = self.dir - va.scale(self.dir.dot(&va));
        let dp = self.origin - *pa;
        let dpva = dp - va.scale(dp.dot(&va));

        let a = vl.norm_squared();
        let b = 2.0 * vl.dot(&dpva);
        let c = dpva.norm_squared() - r * r;

        // Get roots for cylinder surfaces
        if let Some(cylinder_roots) = solve_quadratic(a, b, c) {
            match kind {
                CylinderKind::Infinite => Some(IntersectionResult::from_slice(&cylinder_roots)),
                CylinderKind::Capped => {
                    let mut result = IntersectionResult::from_slice(&cylinder_roots);
                    // In case of cylinder with caps we have to check intersection with caps
                    for (cap_center, cap_normal) in [(pa, -va), (pb, va)].iter() {
                        let cap_plane =
                            Plane::from_normal_and_point(cap_normal, cap_center).unwrap();
                        let t = self.plane_intersection(&cap_plane);
                        if t > 0.0 {
                            let intersection = self.get_point(t);
                            if (*cap_center - intersection).norm_squared() <= r * r {
                                // Point inside cap bounds
                                result.merge(t);
                            }
                        }
                    }
                    result.merge_slice(&cylinder_roots);
                    Some(result)
                }
                CylinderKind::Finite => {
                    // In case of finite cylinder without caps we have to check that intersection
                    // points on cylinder surface are between two planes of caps.
                    let mut result = None;
                    for root in cylinder_roots.iter() {
                        let int_point = self.get_point(*root);
                        if (int_point - *pa).dot(&va) >= 0.0 && (*pb - int_point).dot(&va) >= 0.0 {
                            match &mut result {
                                None => {
                                    result = Some(IntersectionResult {
                                        min: *root,
                                        max: *root,
                                    })
                                }
                                Some(result) => result.merge(*root),
                            }
                        }
                    }
                    result
                }
            }
        } else {
            // We have no roots, so no intersection.
            None
        }
    }

    #[inline]
    pub fn try_eval_points(&self, result: Option<IntersectionResult>) -> Option<[Vector3<f32>; 2]> {
        match result {
            None => None,
            Some(result) => {
                let a = if result.min >= 0.0 && result.min <= 1.0 {
                    Some(self.get_point(result.min))
                } else {
                    None
                };

                let b = if result.max >= 0.0 && result.max <= 1.0 {
                    Some(self.get_point(result.max))
                } else {
                    None
                };

                match a {
                    None => b.map(|b| [b, b]),
                    Some(a) => match b {
                        None => Some([a, a]),
                        Some(b) => Some([a, b]),
                    },
                }
            }
        }
    }

    #[inline]
    pub fn capsule_intersection(
        &self,
        pa: &Vector3<f32>,
        pb: &Vector3<f32>,
        radius: f32,
    ) -> Option<[Vector3<f32>; 2]> {
        // Dumb approach - check intersection with finite cylinder without caps,
        // then check two sphere caps.
        let cylinder = self.cylinder_intersection(pa, pb, radius, CylinderKind::Finite);
        let cap_a = self.sphere_intersection(pa, radius);
        let cap_b = self.sphere_intersection(pb, radius);
        self.try_eval_points(IntersectionResult::from_set(&[cylinder, cap_a, cap_b]))
    }

    /// Transforms ray using given matrix. This method is useful when you need to
    /// transform ray into some object space to simplify calculations. For example
    /// you may have mesh with lots of triangles, and in one way you would take all
    /// vertices, transform them into world space by some matrix, then do intersection
    /// test in world space. This works, but too inefficient, much more faster would
    /// be to put ray into object space and do intersection test in object space. This
    /// removes vertex*matrix multiplication and significantly improves performance.
    #[must_use = "Method does not modify ray, instead it returns transformed copy"]
    #[inline]
    pub fn transform(&self, mat: Matrix4<f32>) -> Self {
        Self {
            origin: mat.transform_point(&Point3::from(self.origin)).coords,
            dir: mat.transform_vector(&self.dir),
        }
    }
}

#[cfg(test)]
mod test {
    use nalgebra::Matrix4;

    use crate::{
        aabb::AxisAlignedBoundingBox,
        plane::Plane,
        ray::{CylinderKind, Ray},
        Vector3,
    };

    use super::IntersectionResult;

    #[test]
    fn intersection() {
        let triangle = [
            Vector3::new(0.0, 0.5, 0.0),
            Vector3::new(-0.5, -0.5, 0.0),
            Vector3::new(0.5, -0.5, 0.0),
        ];
        let ray = Ray::from_two_points(Vector3::new(0.0, 0.0, -2.0), Vector3::new(0.0, 0.0, -1.0));
        assert!(ray.triangle_intersection_point(&triangle).is_none());
    }

    #[test]
    fn default_for_ray() {
        let ray = Ray::default();
        assert_eq!(ray.origin, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(ray.dir, Vector3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn intersection_result_from_slice() {
        let ir = IntersectionResult::from_slice(&[0.0, -1.0, 1.0]);
        assert_eq!(ir.min, -1.0);
        assert_eq!(ir.max, 1.0);
    }

    #[test]
    fn intersection_result_from_set() {
        assert!(IntersectionResult::from_set(&[None, None]).is_none());

        let ir = IntersectionResult::from_set(&[
            Some(IntersectionResult {
                min: -1.0,
                max: 0.0,
            }),
            Some(IntersectionResult { min: 0.0, max: 1.0 }),
        ]);
        assert!(ir.is_some());
        assert_eq!(ir.unwrap().min, -1.0);
        assert_eq!(ir.unwrap().max, 1.0);
    }

    #[test]
    fn intersection_result_merge() {
        let mut ir = IntersectionResult {
            min: -1.0,
            max: 1.0,
        };
        ir.merge(-10.0);
        ir.merge(10.0);

        assert_eq!(ir.min, -10.0);
        assert_eq!(ir.max, 10.0);
    }

    #[test]
    fn intersection_result_merge_slice() {
        let mut ir = IntersectionResult {
            min: -1.0,
            max: 1.0,
        };
        ir.merge_slice(&[-10.0, 0.0, 10.0]);

        assert_eq!(ir.min, -10.0);
        assert_eq!(ir.max, 10.0);
    }

    #[test]
    fn ray_new() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert_eq!(ray.origin, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(ray.dir, Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn ray_try_eval_points() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert!(ray.try_eval_points(None).is_none());

        let ir = IntersectionResult { min: 0.0, max: 1.0 };
        assert_eq!(
            ray.try_eval_points(Some(ir)),
            Some([Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0)])
        );

        let ir = IntersectionResult {
            min: -1.0,
            max: 1.0,
        };
        assert_eq!(
            ray.try_eval_points(Some(ir)),
            Some([Vector3::new(1.0, 1.0, 1.0), Vector3::new(1.0, 1.0, 1.0)])
        );

        let ir = IntersectionResult {
            min: 0.0,
            max: 10.0,
        };
        assert_eq!(
            ray.try_eval_points(Some(ir)),
            Some([Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0)])
        );

        let ir = IntersectionResult {
            min: -10.0,
            max: 10.0,
        };
        assert_eq!(ray.try_eval_points(Some(ir)), None);
    }

    #[test]
    fn ray_sphere_intersection() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0));

        assert!(ray
            .sphere_intersection(&Vector3::new(-10.0, -10.0, -10.0), 1.0)
            .is_none());

        let result = ray.sphere_intersection(&Vector3::new(0.0, 0.0, 0.0), 1.0);
        assert_eq!(result.unwrap().min, -1.0);
        assert_eq!(result.unwrap().max, 1.0);
    }

    #[test]
    fn ray_sphere_intersection_points() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0));

        assert!(ray
            .sphere_intersection_points(&Vector3::new(-10.0, -10.0, -10.0), 1.0)
            .is_none());

        assert_eq!(
            ray.sphere_intersection_points(&Vector3::new(0.0, 0.0, 0.0), 1.0),
            Some([Vector3::new(1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0)])
        );
    }

    #[test]
    fn ray_is_intersect_sphere() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0));

        assert!(!ray.is_intersect_sphere(&Vector3::new(-10.0, -10.0, -10.0), 1.0));
        assert!(ray.is_intersect_sphere(&Vector3::new(0.0, 0.0, 0.0), 1.0));
    }

    #[test]
    fn ray_project_point() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert_eq!(ray.project_point(&Vector3::new(0.0, 0.0, 0.0)), 0.0);
        assert_eq!(ray.project_point(&Vector3::new(1.0, 0.0, 0.0)), 0.33333334);
        assert_eq!(ray.project_point(&Vector3::new(0.0, 1.0, 0.0)), 0.33333334);
        assert_eq!(ray.project_point(&Vector3::new(0.0, 0.0, 1.0)), 0.33333334);
        assert_eq!(ray.project_point(&Vector3::new(1.0, 1.0, 1.0)), 1.0);
    }

    #[test]
    fn ray_get_point() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert_eq!(ray.get_point(0.0), Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(ray.get_point(10.0), Vector3::new(10.0, 10.0, 10.0));
    }

    #[test]
    fn ray_box_intersection() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));
        let ir = ray.box_intersection(
            &Vector3::new(1.0, 1.0, 1.0),
            &Vector3::new(10.0, 10.0, 10.0),
        );
        assert_eq!(ir.unwrap().min, 1.0);
        assert_eq!(ir.unwrap().max, 10.0);

        assert!(ray
            .box_intersection(&Vector3::new(1.0, 1.0, 0.0), &Vector3::new(10.0, 10.0, 0.0))
            .is_none());
        assert!(ray
            .box_intersection(&Vector3::new(1.0, 0.0, 1.0), &Vector3::new(10.0, 0.0, 10.0))
            .is_none());

        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(-1.0, -1.0, -1.0));
        let ir = ray.box_intersection(
            &Vector3::new(-10.0, -10.0, -10.0),
            &Vector3::new(-1.0, -1.0, -1.0),
        );
        assert_eq!(ir.unwrap().min, 1.0);
        assert_eq!(ir.unwrap().max, 10.0);
    }

    #[test]
    fn ray_box_intersection_points() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert!(ray
            .box_intersection_points(&Vector3::new(1.0, 1.0, 0.0), &Vector3::new(10.0, 10.0, 0.0))
            .is_none());
        assert_eq!(
            ray.box_intersection_points(
                &Vector3::new(1.0, 1.0, 1.0),
                &Vector3::new(10.0, 10.0, 10.0)
            ),
            Some([Vector3::new(1.0, 1.0, 1.0), Vector3::new(1.0, 1.0, 1.0)])
        );
    }

    #[test]
    fn ray_aabb_intersection() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert!(ray
            .aabb_intersection(&AxisAlignedBoundingBox {
                min: Vector3::new(1.0, 1.0, 0.0),
                max: Vector3::new(10.0, 10.0, 0.0)
            })
            .is_none());

        let ir = ray.aabb_intersection(&AxisAlignedBoundingBox {
            min: Vector3::new(1.0, 1.0, 1.0),
            max: Vector3::new(10.0, 10.0, 10.0),
        });
        assert_eq!(ir.unwrap().min, 1.0);
        assert_eq!(ir.unwrap().max, 10.0);
    }

    #[test]
    fn ray_aabb_intersection_points() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert!(ray
            .aabb_intersection_points(&AxisAlignedBoundingBox {
                min: Vector3::new(1.0, 1.0, 0.0),
                max: Vector3::new(10.0, 10.0, 0.0)
            })
            .is_none());

        assert_eq!(
            ray.aabb_intersection_points(&AxisAlignedBoundingBox {
                min: Vector3::new(1.0, 1.0, 1.0),
                max: Vector3::new(10.0, 10.0, 10.0),
            }),
            Some([Vector3::new(1.0, 1.0, 1.0), Vector3::new(1.0, 1.0, 1.0)])
        );
    }

    #[test]
    fn ray_plane_intersection() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert_eq!(
            ray.plane_intersection(
                &Plane::from_normal_and_point(
                    &Vector3::new(1.0, 1.0, 1.0),
                    &Vector3::new(0.0, 0.0, 0.0)
                )
                .unwrap()
            ),
            0.0
        );
    }

    #[test]
    fn ray_plane_intersection_point() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert_eq!(
            ray.plane_intersection_point(
                &Plane::from_normal_and_point(
                    &Vector3::new(1.0, 1.0, 1.0),
                    &Vector3::new(0.0, 0.0, 0.0)
                )
                .unwrap()
            ),
            Some(Vector3::new(0.0, 0.0, 0.0))
        );

        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 0.0));

        assert_eq!(
            ray.plane_intersection_point(
                &Plane::from_normal_and_point(
                    &Vector3::new(0.0, 0.0, 1.0),
                    &Vector3::new(1.0, 1.0, 1.0),
                )
                .unwrap()
            ),
            None
        );
    }

    #[test]
    fn ray_triangle_intersection() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert_eq!(
            ray.triangle_intersection(&[
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(1.0, 1.0, 0.0),
            ]),
            Some((0.0, Vector3::new(0.0, 0.0, 0.0)))
        );

        assert_eq!(
            ray.triangle_intersection(&[
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(1.0, -1.0, 0.0),
                Vector3::new(-1.0, -1.0, 0.0),
            ]),
            None
        );
    }

    #[test]
    fn ray_triangle_intersection_point() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert_eq!(
            ray.triangle_intersection_point(&[
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(1.0, 1.0, 0.0),
            ]),
            Some(Vector3::new(0.0, 0.0, 0.0))
        );

        assert_eq!(
            ray.triangle_intersection_point(&[
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(1.0, -1.0, 0.0),
                Vector3::new(-1.0, -1.0, 0.0),
            ]),
            None
        );
    }

    #[test]
    fn ray_cylinder_intersection() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        // Infinite
        let ir = ray.cylinder_intersection(
            &Vector3::new(0.0, 0.0, 0.0),
            &Vector3::new(1.0, 0.0, 0.0),
            1.0,
            CylinderKind::Infinite,
        );
        assert_eq!(ir.unwrap().min, -0.70710677);
        assert_eq!(ir.unwrap().max, 0.70710677);

        // Finite
        let ir = ray.cylinder_intersection(
            &Vector3::new(0.0, 0.0, 0.0),
            &Vector3::new(1.0, 0.0, 0.0),
            1.0,
            CylinderKind::Finite,
        );
        assert_eq!(ir.unwrap().min, 0.70710677);
        assert_eq!(ir.unwrap().max, 0.70710677);

        // Capped
        let ir = ray.cylinder_intersection(
            &Vector3::new(0.0, 0.0, 0.0),
            &Vector3::new(1.0, 0.0, 0.0),
            1.0,
            CylinderKind::Capped,
        );
        assert_eq!(ir.unwrap().min, -0.70710677);
        assert_eq!(ir.unwrap().max, 0.70710677);
    }

    #[test]
    fn ray_capsule_intersection() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        assert_eq!(
            ray.capsule_intersection(
                &Vector3::new(0.0, 0.0, 0.0),
                &Vector3::new(1.0, 0.0, 0.0),
                1.0,
            ),
            Some([
                Vector3::new(0.70710677, 0.70710677, 0.70710677),
                Vector3::new(0.70710677, 0.70710677, 0.70710677)
            ])
        );
        assert_eq!(
            ray.capsule_intersection(
                &Vector3::new(10.0, 0.0, 0.0),
                &Vector3::new(11.0, 0.0, 0.0),
                1.0,
            ),
            None
        );
    }

    #[test]
    fn ray_transform() {
        let ray = Ray::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 1.0));

        let new_ray = ray.transform(Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ));

        assert_eq!(ray.origin, new_ray.origin);
        assert_eq!(ray.dir, new_ray.dir);
    }
}
