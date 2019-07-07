pub mod vec2;
pub mod vec3;
pub mod vec4;
pub mod mat4;
pub mod quat;
pub mod ray;
pub mod plane;
pub mod triangulator;

use serde::{Serialize, Deserialize};
use vec2::*;
use vec3::*;

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}

impl<T> Rect<T> where T: Default {
    pub fn new(x: T, y: T, w: T, h: T) -> Rect<T> {
        Rect { x, y, w, h }
    }

    pub fn default() -> Rect<T> {
        Rect {
            x: T::default(),
            y: T::default(),
            w: T::default(),
            h: T::default(),
        }
    }
}

#[derive(Copy, Clone)]
pub enum PlaneClass {
    XY,
    YZ,
    XZ,
}

pub fn classify_plane(normal: Vec3) -> PlaneClass {
    let mut longest = 0.0f32;
    let mut class = PlaneClass::XY;

    if normal.x.abs() > longest {
        longest = normal.x.abs();
        class = PlaneClass::YZ;
    }

    if normal.y.abs() > longest {
        longest = normal.y.abs();
        class = PlaneClass::XZ;
    }

    if normal.z.abs() > longest {
        class = PlaneClass::XY;
    }

    class
}

pub fn get_polygon_normal(polygon: &[Vec3]) -> Result<Vec3, &'static str> {
    let mut normal = Vec3::new();

    for (i, current) in polygon.iter().enumerate() {
        let next = polygon[(i + 1) % polygon.len()];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }

    if normal.sqr_len() > std::f32::EPSILON {
        return Ok(normal.normalized_unchecked());
    }

    Err("Unable to get normal of degenerated polygon!")
}

pub fn get_signed_triangle_area(v1: Vec2, v2: Vec2, v3: Vec2) -> f32 {
    0.5 * (v1.x * (v3.y - v2.y) + v2.x * (v1.y - v3.y) + v3.x * (v2.y - v1.y))
}

pub fn vec3_to_vec2_by_plane(plane_class: PlaneClass, normal: Vec3, point: Vec3) -> Vec2 {
    match plane_class {
        PlaneClass::XY => if normal.z < 0.0 { Vec2::make(point.y, point.x) } else { Vec2::make(point.x, point.y) }
        PlaneClass::XZ => if normal.y < 0.0 { Vec2::make(point.x, point.z) } else { Vec2::make(point.z, point.x) }
        PlaneClass::YZ => if normal.x < 0.0 { Vec2::make(point.z, point.y) } else { Vec2::make(point.y, point.z) }
    }
}

pub fn is_point_inside_2d_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let ba = b - a;
    let ca = c - a;

    let vp = point - a;

    let ba_dot_ba = ba.dot(ba);
    let ca_dot_ba = ca.dot(ba);
    let ca_dot_ca = ca.dot(ca);

    let dot_02 = ca.dot(vp);
    let dot_12 = ba.dot(vp);

    let inv_denom = 1.0 / (ca_dot_ca * ba_dot_ba - ca_dot_ba * ca_dot_ba);

    // calculate barycentric coordinates
    let u = (ba_dot_ba * dot_02 - ca_dot_ba * dot_12) * inv_denom;
    let v = (ca_dot_ca * dot_12 - ca_dot_ba * dot_02) * inv_denom;

    (u >= 0.0) && (v >= 0.0) && (u + v < 1.0)
}
