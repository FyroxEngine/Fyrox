use crate::math::vec3::Vec3;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Default for Plane {
    fn default() -> Self {
        Plane {
            normal: Vec3::make(0.0, 1.0, 0.0),
            d: 0.0
        }
    }
}

impl Plane {
    /// Creates plane from a point and normal vector at that point.
    /// May fail if normal is degenerated vector.
    #[inline]
    pub fn from_normal_and_point(normal: Vec3, point: Vec3) -> Option<Plane> {
        if let Some(normalized_normal) = normal.normalized() {
            return Some(Plane {
                normal: normalized_normal,
                d: -point.dot(&normalized_normal),
            });
        }
        None
    }

    #[inline]
    pub fn dot(&self, point: Vec3) -> f32 {
        self.normal.dot(&point) + self.d
    }

    #[inline]
    pub fn distance(&self, point: Vec3) -> f32 {
        self.dot(point).abs()
    }
}

#[test]
fn plane_sanity_tests() {
    // Computation test
    let plane = Plane::from_normal_and_point(
        Vec3::make(0.0, 10.0, 0.0), Vec3::make(0.0, 3.0, 0.0));
    assert!(plane.is_some());
    let plane = plane.unwrap();
    assert_eq!(plane.normal.x, 0.0);
    assert_eq!(plane.normal.y, 1.0);
    assert_eq!(plane.normal.z, 0.0);
    assert_eq!(plane.d, -3.0);

    // Degenerated normal case
    let plane = Plane::from_normal_and_point(
        Vec3::make(0.0, 0.0, 0.0), Vec3::make(0.0, 0.0, 0.0));
    assert!(plane.is_none());
}