// Clippy complains about normal mathematical symbols like A, B, C for quadratic equation.
#![allow(clippy::many_single_char_names)]

use crate::math::{
    plane::Plane,
    vec3::Vec3,
    is_point_inside_triangle,
    solve_quadratic,
};

pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Default for Ray {
    fn default() -> Self {
        Ray {
            origin: Vec3::default(),
            dir: Vec3::new(0.0, 0.0, 1.0),
        }
    }
}

/// Pair of ray equation parameters.
pub struct IntersectionResult {
    pub min: f32,
    pub max: f32,
}

impl Default for IntersectionResult {
    fn default() -> Self {
        Self {
            min: std::f32::MAX,
            max: -std::f32::MAX,
        }
    }
}

impl IntersectionResult {
    /// Updates min and max ray equation parameters according to a new parameter -
    /// expands range if `param` was outside of that range.
    pub fn push(&mut self, param: f32) {
        if param < self.min {
            self.min = param;
        }
        if param > self.max {
            self.max = param;
        }
    }

    pub fn push_slice(&mut self, params: &[f32]) {
        for param in params {
            self.push(*param)
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
    pub fn from_two_points(begin: &Vec3, end: &Vec3) -> Option<Ray> {
        let dir = *end - *begin;
        if dir.len() >= std::f32::EPSILON {
            Some(Ray { origin: *begin, dir })
        } else {
            None
        }
    }

    /// Checks intersection with sphere. Returns two intersection points or none
    /// if there was no intersection.
    #[inline]
    pub fn sphere_intersection_points(&self, position: &Vec3, radius: f32) -> Option<[Vec3; 2]> {
        let mut result = IntersectionResult::default();
        if self.sphere_intersection(position, radius, &mut result) {
            Some([self.get_point(result.min), self.get_point(result.max)])
        } else {
            None
        }
    }

    pub fn sphere_intersection(&self, position: &Vec3, radius: f32, result: &mut IntersectionResult) -> bool {
        let d = self.origin - *position;
        let a = self.dir.dot(&self.dir);
        let b = 2.0 * self.dir.dot(&d);
        let c = d.dot(&d) - radius * radius;
        if let Some(roots) = solve_quadratic(a, b, c) {
            result.push_slice(&roots);
            true
        } else {
            false
        }
    }

    /// Checks intersection with sphere.
    #[inline]
    pub fn is_intersect_sphere(&self, position: Vec3, radius: f32) -> bool {
        let d = self.origin - position;
        let a = self.dir.dot(&self.dir);
        let b = 2.0 * self.dir.dot(&d);
        let c = d.dot(&d) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;
        discriminant >= 0.0
    }

    /// Returns t factor (at pt=o+d*t equation) for projection of given point at ray
    #[inline]
    pub fn project_point(&self, point: Vec3) -> f32 {
        (point - self.origin).dot(&self.dir) / self.dir.sqr_len()
    }

    /// Returns point on ray which defined by pt=o+d*t equation.
    #[inline]
    pub fn get_point(&self, t: f32) -> Vec3 {
        self.origin + self.dir.scale(t)
    }

    pub fn box_intersection(&self, min: &Vec3, max: &Vec3, result: &mut IntersectionResult) -> bool {
        let (mut tmin, mut tmax) = if self.dir.x >= 0.0 {
            ((min.x - self.origin.x) / self.dir.x,
             (max.x - self.origin.x) / self.dir.x)
        } else {
            ((max.x - self.origin.x) / self.dir.x,
             (min.x - self.origin.x) / self.dir.x)
        };

        let (tymin, tymax) = if self.dir.y >= 0.0 {
            ((min.y - self.origin.y) / self.dir.y,
             (max.y - self.origin.y) / self.dir.y)
        } else {
            ((max.y - self.origin.y) / self.dir.y,
             (min.y - self.origin.y) / self.dir.y)
        };

        if tmin > tymax || (tymin > tmax) {
            return false;
        }
        if tymin > tmin {
            tmin = tymin;
        }
        if tymax < tmax {
            tmax = tymax;
        }
        let (tzmin, tzmax) = if self.dir.z >= 0.0 {
            ((min.z - self.origin.z) / self.dir.z,
             (max.z - self.origin.z) / self.dir.z)
        } else {
            ((max.z - self.origin.z) / self.dir.z,
             (min.z - self.origin.z) / self.dir.z)
        };

        if (tmin > tzmax) || (tzmin > tmax) {
            return false;
        }
        if tzmin > tmin {
            tmin = tzmin;
        }
        if tzmax < tmax {
            tmax = tzmax;
        }
        if tmin < 1.0 && tmax > 0.0 {
            result.push(tmin);
            result.push(tmax);
            true
        } else {
            false
        }
    }

    pub fn box_intersection_points(&self, min: &Vec3, max: &Vec3) -> Option<[Vec3; 2]> {
        let mut result = IntersectionResult::default();
        if self.box_intersection(min, max, &mut result) {
            Some([self.get_point(result.min), self.get_point(result.max)])
        } else {
            None
        }
    }

    /// Solves plane equation in order to find ray equation parameter.
    /// There is no intersection if result < 0.
    pub fn plane_intersection(&self, plane: &Plane) -> f32 {
        let u = -(self.origin.dot(&plane.normal) + plane.d);
        let v = self.dir.dot(&plane.normal);
        u / v
    }

    pub fn plane_intersection_point(&self, plane: &Plane) -> Option<Vec3> {
        let t = self.plane_intersection(plane);
        if t < 0.0 {
            None
        } else {
            Some(self.get_point(t))
        }
    }

    pub fn triangle_intersection(&self, vertices: &[Vec3; 3]) -> Option<Vec3> {
        let ba = vertices[1] - vertices[0];
        let ca = vertices[2] - vertices[0];
        let plane = Plane::from_normal_and_point(&ba.cross(&ca), &vertices[0]).ok()?;

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
    pub fn cylinder_intersection(&self, pa: &Vec3, pb: &Vec3, r: f32, kind: CylinderKind, result: &mut IntersectionResult) -> bool {
        let va = *pb - *pa;
        let vl = self.dir - va.scale(self.dir.dot(&va));
        let dp = self.origin - *pa;
        let dpva = dp - va.scale(dp.dot(&va));

        let a = vl.sqr_len();
        let b = 2.0 * vl.dot(&dpva);
        let c = dpva.sqr_len() - r * r;

        // Get roots for cylinder surfaces
        if let Some(cylinder_roots) = solve_quadratic(a, b, c) {
            match kind {
                CylinderKind::Infinite => result.push_slice(&cylinder_roots),
                CylinderKind::Capped => {
                    // In case of cylinder with caps we have to check intersection with caps
                    for (cap_center, cap_normal) in [(pa, -va), (pb, va)].iter() {
                        let cap_plane = Plane::from_normal_and_point(cap_normal, cap_center).unwrap();
                        let t = self.plane_intersection(&cap_plane);
                        if t > 0.0 {
                            let intersection = self.get_point(t);
                            if cap_center.sqr_distance(&intersection) <= r * r {
                                // Point inside cap bounds
                                result.push(t);
                            }
                        }
                    }
                    result.push_slice(&cylinder_roots);
                }
                CylinderKind::Finite => {
                    // In case of finite cylinder without caps we have to check that intersection
                    // points on cylinder surface are between two planes of caps.
                    for root in cylinder_roots.iter() {
                        let int_point = self.get_point(*root);
                        if (int_point - *pa).dot(&va) >= 0.0 && (*pb - int_point).dot(&va) >= 0.0 {
                            result.push(*root);
                        }
                    }
                }
            }

            true
        } else {
            // We have no roots, so no intersection.
            false
        }
    }

    pub fn capsule_intersection(&self, pa: &Vec3, pb: &Vec3, radius: f32) -> Option<[Vec3; 2]> {
        // Dumb approach - check intersection with finite cylinder without caps, then check
        // two sphere caps.
        let mut result = IntersectionResult::default();

        if self.cylinder_intersection(pa, pb, radius, CylinderKind::Finite, &mut result) ||
            self.sphere_intersection(pa, radius, &mut result) ||
            self.sphere_intersection(pb, radius, &mut result) {
            Some([self.get_point(result.min), self.get_point(result.max)])
        } else {
            None
        }
    }
}
