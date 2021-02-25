// Clippy complains about normal mathematical symbols like A, B, C for quadratic equation.
#![allow(clippy::many_single_char_names)]

use crate::algebra::{Matrix4, Point3, Vector3};
use crate::math::aabb::AxisAlignedBoundingBox;
use crate::math::{is_point_inside_triangle, plane::Plane, solve_quadratic};

#[derive(Copy, Clone, Debug)]
pub struct Ray {
    pub origin: Vector3<f32>,
    pub dir: Vector3<f32>,
}

impl Default for Ray {
    fn default() -> Self {
        Ray {
            origin: Vector3::new(0.0, 0.0, 0.0),
            dir: Vector3::new(0.0, 0.0, 1.0),
        }
    }
}

/// Pair of ray equation parameters.
#[derive(Clone, Debug)]
pub struct IntersectionResult {
    pub min: f32,
    pub max: f32,
}

impl IntersectionResult {
    pub fn from_slice(roots: &[f32]) -> Self {
        let mut min = std::f32::MAX;
        let mut max = -std::f32::MAX;
        for n in roots {
            min = min.min(*n);
            max = max.max(*n);
        }
        Self { min, max }
    }

    pub fn from_set(results: &[Option<IntersectionResult>]) -> Option<Self> {
        let mut result = None;
        for v in results {
            match result {
                None => result = v.clone(),
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
    pub fn merge(&mut self, param: f32) {
        if param < self.min {
            self.min = param;
        }
        if param > self.max {
            self.max = param;
        }
    }

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

    pub fn sphere_intersection(
        &self,
        position: &Vector3<f32>,
        radius: f32,
    ) -> Option<IntersectionResult> {
        let d = self.origin - *position;
        let a = self.dir.dot(&self.dir);
        let b = 2.0 * self.dir.dot(&d);
        let c = d.dot(&d) - radius * radius;
        if let Some(roots) = solve_quadratic(a, b, c) {
            Some(IntersectionResult::from_slice(&roots))
        } else {
            None
        }
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

    pub fn box_intersection_points(
        &self,
        min: &Vector3<f32>,
        max: &Vector3<f32>,
    ) -> Option<[Vector3<f32>; 2]> {
        self.try_eval_points(self.box_intersection(min, max))
    }

    pub fn aabb_intersection(&self, aabb: &AxisAlignedBoundingBox) -> Option<IntersectionResult> {
        self.box_intersection(&aabb.min, &aabb.max)
    }

    pub fn aabb_intersection_points(
        &self,
        aabb: &AxisAlignedBoundingBox,
    ) -> Option<[Vector3<f32>; 2]> {
        self.box_intersection_points(&aabb.min, &aabb.max)
    }

    /// Solves plane equation in order to find ray equation parameter.
    /// There is no intersection if result < 0.
    pub fn plane_intersection(&self, plane: &Plane) -> f32 {
        let u = -(self.origin.dot(&plane.normal) + plane.d);
        let v = self.dir.dot(&plane.normal);
        u / v
    }

    pub fn plane_intersection_point(&self, plane: &Plane) -> Option<Vector3<f32>> {
        let t = self.plane_intersection(plane);
        if !(0.0..=1.0).contains(&t) {
            None
        } else {
            Some(self.get_point(t))
        }
    }

    pub fn triangle_intersection(&self, vertices: &[Vector3<f32>; 3]) -> Option<Vector3<f32>> {
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
    /// https://mrl.nyu.edu/~dzorin/rend05/lecture2.pdf
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
    pub fn cylinder_intersection(
        &self,
        pa: &Vector3<f32>,
        pb: &Vector3<f32>,
        r: f32,
        kind: CylinderKind,
    ) -> Option<IntersectionResult> {
        let va = (*pb - *pa)
            .try_normalize(std::f32::EPSILON)
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
                    None => match b {
                        None => None,
                        Some(b) => Some([b, b]),
                    },
                    Some(a) => match b {
                        None => Some([a, a]),
                        Some(b) => Some([a, b]),
                    },
                }
            }
        }
    }

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
    pub fn transform(&self, mat: Matrix4<f32>) -> Self {
        Self {
            origin: mat.transform_point(&Point3::from(self.origin)).coords,
            dir: mat.transform_point(&Point3::from(self.dir)).coords,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::math::ray::Ray;
    use crate::math::Vector3;

    #[test]
    fn intersection() {
        let triangle = [
            Vector3::new(0.0, 0.5, 0.0),
            Vector3::new(-0.5, -0.5, 0.0),
            Vector3::new(0.5, -0.5, 0.0),
        ];
        let ray =
            Ray::from_two_points(&Vector3::new(0.0, 0.0, -2.0), &Vector3::new(0.0, 0.0, -1.0))
                .unwrap();
        assert!(ray.triangle_intersection(&triangle).is_none());
    }
}
