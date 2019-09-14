use crate::math::vec3::Vec3;

pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Default for Ray {
    fn default() -> Self {
        Ray {
            origin: Vec3::default(),
            dir: Vec3::make(0.0, 0.0, 1.0),
        }
    }
}

impl Ray {
    /// Creates ray from two points. May fail if begin == end.
    #[inline]
    pub fn from_two_points(begin: Vec3, end: Vec3) -> Option<Ray> {
        let dir = end - begin;
        if dir.len() >= std::f32::EPSILON {
            Some(Ray { origin: begin, dir })
        } else {
            None
        }
    }

    /// Checks intersection with sphere. Returns two intersection points or none
    /// if there was no intersection.
    #[inline]
    pub fn sphere_intersection(&self, position: Vec3, radius: f32) -> Option<(Vec3, Vec3)> {
        let d = self.origin - position;
        let a = self.dir.dot(&self.dir);
        let b = 2.0 * self.dir.dot(&d);
        let c = d.dot(&d) - radius * radius;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return None;
        }
        let discr_root = discriminant.sqrt();
        let r1 = (-b + discr_root) / 2.0;
        let r2 = (-b - discr_root) / 2.0;
        Some((self.origin + self.dir.scale(r1), self.origin + self.dir.scale(r2)))
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
}
