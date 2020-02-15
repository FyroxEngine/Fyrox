use crate::{
    math::vec3::Vec3,
    visitor::{Visit, Visitor, VisitResult},
};

#[derive(Copy, Clone)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Default for Plane {
    fn default() -> Self {
        Plane {
            normal: Vec3::new(0.0, 1.0, 0.0),
            d: 0.0,
        }
    }
}

impl Plane {
    /// Creates plane from a point and normal vector at that point.
    /// May fail if normal is degenerated vector.
    #[inline]
    pub fn from_normal_and_point(normal: &Vec3, point: &Vec3) -> Result<Self, ()> {
        if let Some(normalized_normal) = normal.normalized() {
            Ok(Self {
                normal: normalized_normal,
                d: -point.dot(&normalized_normal),
            })
        } else {
            Err(())
        }
    }

    /// Creates plane using coefficients of plane equation Ax + By + Cz + D = 0
    /// May fail if length of normal vector is zero (normal is degenerated vector).
    #[inline]
    pub fn from_abcd(a: f32, b: f32, c: f32, d: f32) -> Result<Self, ()> {
        let normal = Vec3::new(a, b, c);
        let len = normal.len();
        if len == 0.0 {
            Err(())
        } else {
            let k = 1.0 / len;
            Ok(Self {
                normal: normal.scale(k),
                d: d * k,
            })
        }
    }

    #[inline]
    pub fn dot(&self, point: &Vec3) -> f32 {
        self.normal.dot(&point) + self.d
    }

    #[inline]
    pub fn distance(&self, point: &Vec3) -> f32 {
        self.dot(point).abs()
    }

    /// http://geomalgorithms.com/a05-_intersect-1.html
    pub fn intersection_point(&self, b: &Plane, c: &Plane) -> Vec3 {
        let f = -1.0 / self.normal.dot(&b.normal.cross(&c.normal));

        let v1 = b.normal.cross(&c.normal).scale(self.d);
        let v2 = c.normal.cross(&self.normal).scale(b.d);
        let v3 = self.normal.cross(&b.normal).scale(c.d);

        (v1 + v2 + v3).scale(f)
    }
}

impl Visit for Plane {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.normal.visit("Normal", visitor)?;
        self.d.visit("D", visitor)?;

        visitor.leave_region()
    }
}

#[test]
fn plane_sanity_tests() {
    // Computation test
    let plane = Plane::from_normal_and_point(
        &Vec3::new(0.0, 10.0, 0.0), &Vec3::new(0.0, 3.0, 0.0));
    assert!(plane.is_ok());
    let plane = plane.unwrap();
    assert_eq!(plane.normal.x, 0.0);
    assert_eq!(plane.normal.y, 1.0);
    assert_eq!(plane.normal.z, 0.0);
    assert_eq!(plane.d, -3.0);

    // Degenerated normal case
    let plane = Plane::from_normal_and_point(
        &Vec3::new(0.0, 0.0, 0.0), &Vec3::new(0.0, 0.0, 0.0));
    assert!(plane.is_err());

    let plane = Plane::from_abcd(0.0, 0.0, 0.0, 0.0);
    assert!(plane.is_err())
}