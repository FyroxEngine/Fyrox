pub mod vec2;
pub mod vec3;
pub mod vec4;
pub mod mat4;
pub mod quat;
pub mod ray;
pub mod plane;
pub mod triangulator;

use vec2::*;
use vec3::*;
use std::ops::{Add, Sub, Mul};
use crate::utils::visitor::{Visit, VisitResult, Visitor};

#[derive(Copy, Clone, Debug)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub w: T,
    pub h: T,
}

impl<T> Rect<T> where T: Default + Add<Output=T> + Sub<Output=T> + Mul<Output=T> + Copy {
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

    pub fn inflate(&self, dw: T, dh: T) -> Rect<T> {
        Rect {
            x: self.x - dw,
            y: self.y - dh,
            w: self.w + dw + dw,
            h: self.h + dh + dh,
        }
    }

    pub fn deflate(&self, dw: T, dh: T) -> Rect<T> {
        Rect {
            x: self.x + dw,
            y: self.y + dh,
            w: self.w - (dw + dw),
            h: self.h - (dh + dh),
        }
    }
}

impl<T> Visit for Rect<T> where T: Default + Visit + 'static {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.x.visit("X", visitor)?;
        self.y.visit("Y", visitor)?;
        self.w.visit("W", visitor)?;
        self.h.visit("H", visitor)?;

        visitor.leave_region()
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

pub fn is_point_inside_2d_triangle(point: Vec2, pt_a: Vec2, pt_b: Vec2, pt_c: Vec2) -> bool {
    let ba = pt_b - pt_a;
    let ca = pt_c - pt_a;

    let vp = point - pt_a;

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


pub fn clampf(v: f32, min: f32, max: f32) -> f32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

pub fn wrapf(mut n: f32, mut min_limit: f32, mut max_limit: f32) -> f32 {
    if n >= min_limit && n <= max_limit {
        return n;
    }

    if max_limit == 0.0 && min_limit == 0.0 {
        return 0.0;
    }

    max_limit = max_limit - min_limit;

    let offset = min_limit;
    min_limit = 0.0;
    n = n - offset;

    let num_of_max = (n / max_limit).abs().floor();

    if n >= max_limit {
        n = n - num_of_max * max_limit;
    } else if n < min_limit {
        n = ((num_of_max + 1.0) * max_limit) + n;
    }

    return n + offset;
}

pub fn lerpf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}