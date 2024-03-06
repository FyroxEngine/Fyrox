use nalgebra::Vector3;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Plane {
    pub normal: Vector3<f32>,
    pub d: f32,
}

impl Default for Plane {
    #[inline]
    fn default() -> Self {
        Plane {
            normal: Vector3::new(0.0, 1.0, 0.0),
            d: 0.0,
        }
    }
}

impl Plane {
    /// Creates plane from a point and normal vector at that point.
    /// May fail if normal is degenerated vector.
    #[inline]
    pub fn from_normal_and_point(normal: &Vector3<f32>, point: &Vector3<f32>) -> Option<Self> {
        normal
            .try_normalize(f32::EPSILON)
            .map(|normalized_normal| Self {
                normal: normalized_normal,
                d: -point.dot(&normalized_normal),
            })
    }

    /// Tries to create a plane from three points (triangle). May fail if the triangle is degenerated
    /// (collapsed into a point or a line).
    #[inline]
    pub fn from_triangle(a: &Vector3<f32>, b: &Vector3<f32>, c: &Vector3<f32>) -> Option<Self> {
        let normal = (b - a).cross(&(c - a));
        Self::from_normal_and_point(&normal, a)
    }

    /// Creates plane using coefficients of plane equation Ax + By + Cz + D = 0
    /// May fail if length of normal vector is zero (normal is degenerated vector).
    #[inline]
    pub fn from_abcd(a: f32, b: f32, c: f32, d: f32) -> Option<Self> {
        let normal = Vector3::new(a, b, c);
        let len = normal.norm();
        if len == 0.0 {
            None
        } else {
            let coeff = 1.0 / len;
            Some(Self {
                normal: normal.scale(coeff),
                d: d * coeff,
            })
        }
    }

    #[inline]
    pub fn dot(&self, point: &Vector3<f32>) -> f32 {
        self.normal.dot(point) + self.d
    }

    #[inline]
    pub fn distance(&self, point: &Vector3<f32>) -> f32 {
        self.dot(point).abs()
    }

    /// Projects the given point onto the plane along the normal vector of the plane.
    #[inline]
    pub fn project(&self, point: &Vector3<f32>) -> Vector3<f32> {
        point - self.normal.scale(self.normal.dot(point) + self.d)
    }

    /// <http://geomalgorithms.com/a05-_intersect-1.html>
    pub fn intersection_point(&self, b: &Plane, c: &Plane) -> Vector3<f32> {
        let f = -1.0 / self.normal.dot(&b.normal.cross(&c.normal));

        let v1 = b.normal.cross(&c.normal).scale(self.d);
        let v2 = c.normal.cross(&self.normal).scale(b.d);
        let v3 = self.normal.cross(&b.normal).scale(c.d);

        (v1 + v2 + v3).scale(f)
    }
}

#[cfg(test)]
mod test {
    use crate::plane::Plane;
    use nalgebra::Vector3;

    #[test]
    fn plane_sanity_tests() {
        // Computation test
        let plane = Plane::from_normal_and_point(
            &Vector3::new(0.0, 10.0, 0.0),
            &Vector3::new(0.0, 3.0, 0.0),
        );
        assert!(plane.is_some());
        let plane = plane.unwrap();
        assert_eq!(plane.normal.x, 0.0);
        assert_eq!(plane.normal.y, 1.0);
        assert_eq!(plane.normal.z, 0.0);
        assert_eq!(plane.d, -3.0);

        // Degenerated normal case
        let plane = Plane::from_normal_and_point(
            &Vector3::new(0.0, 0.0, 0.0),
            &Vector3::new(0.0, 0.0, 0.0),
        );
        assert!(plane.is_none());

        let plane = Plane::from_abcd(0.0, 0.0, 0.0, 0.0);
        assert!(plane.is_none())
    }

    #[test]
    fn test_default_for_plane() {
        assert_eq!(
            Plane::default(),
            Plane {
                normal: Vector3::new(0.0, 1.0, 0.0),
                d: 0.0,
            }
        );
    }

    #[test]
    fn test_plane_from_abcd() {
        assert_eq!(Plane::from_abcd(0.0, 0.0, 0.0, 0.0), None);
        assert_eq!(
            Plane::from_abcd(1.0, 1.0, 1.0, 0.0),
            Some(Plane {
                normal: Vector3::new(0.57735026, 0.57735026, 0.57735026),
                d: 0.0
            })
        );
    }

    #[test]
    fn test_plane_dot() {
        let plane = Plane::from_normal_and_point(
            &Vector3::new(0.0, 0.0, 1.0),
            &Vector3::new(0.0, 0.0, 0.0),
        );
        assert!(plane.is_some());
        assert_eq!(plane.unwrap().dot(&Vector3::new(1.0, 1.0, 1.0)), 1.0);
    }

    #[test]
    fn test_plane_distance() {
        let plane = Plane::from_normal_and_point(
            &Vector3::new(0.0, 0.0, 1.0),
            &Vector3::new(0.0, 0.0, 0.0),
        );
        assert!(plane.is_some());
        assert_eq!(plane.unwrap().distance(&Vector3::new(0.0, 0.0, 0.0)), 0.0);
        assert_eq!(plane.unwrap().distance(&Vector3::new(1.0, 0.0, 0.0)), 0.0);
        assert_eq!(plane.unwrap().distance(&Vector3::new(0.0, 1.0, 0.0)), 0.0);
        assert_eq!(plane.unwrap().distance(&Vector3::new(0.0, 0.0, 1.0)), 1.0);
    }

    #[test]
    fn test_plane_intersection_point() {
        let plane = Plane::from_normal_and_point(
            &Vector3::new(0.0, 0.0, 1.0),
            &Vector3::new(0.0, 0.0, 0.0),
        );
        let plane2 = Plane::from_normal_and_point(
            &Vector3::new(0.0, 1.0, 0.0),
            &Vector3::new(0.0, 0.0, 0.0),
        );
        let plane3 = Plane::from_normal_and_point(
            &Vector3::new(1.0, 0.0, 0.0),
            &Vector3::new(0.0, 0.0, 0.0),
        );
        assert!(plane.is_some());
        assert!(plane2.is_some());
        assert!(plane3.is_some());

        assert_eq!(
            plane
                .unwrap()
                .intersection_point(&plane2.unwrap(), &plane3.unwrap()),
            Vector3::new(0.0, 0.0, 0.0)
        );
    }
}
