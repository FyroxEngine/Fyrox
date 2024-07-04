// Clippy complains about normal mathematical symbols like A, B, C for quadratic equation.
#![allow(clippy::many_single_char_names)]

pub mod aabb;
pub mod curve;
pub mod frustum;
pub mod octree;
pub mod plane;
pub mod ray;
pub mod segment;
pub mod triangulator;

use crate::ray::IntersectionResult;
use bytemuck::{Pod, Zeroable};
use nalgebra::{
    Matrix3, Matrix4, RealField, Scalar, SimdRealField, UnitQuaternion, Vector2, Vector3,
};
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
};

pub use rectutils::*;

#[derive(Copy, Clone)]
pub enum PlaneClass {
    XY,
    YZ,
    XZ,
}

#[inline]
#[allow(clippy::useless_let_if_seq)]
pub fn classify_plane(normal: Vector3<f32>) -> PlaneClass {
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

#[inline]
pub fn get_polygon_normal(polygon: &[Vector3<f32>]) -> Result<Vector3<f32>, &'static str> {
    let mut normal = Vector3::default();

    for (i, current) in polygon.iter().enumerate() {
        let next = polygon[(i + 1) % polygon.len()];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }

    normal
        .try_normalize(f32::EPSILON)
        .ok_or("Unable to get normal of degenerated polygon!")
}

#[inline]
pub fn get_signed_triangle_area(v1: Vector2<f32>, v2: Vector2<f32>, v3: Vector2<f32>) -> f32 {
    0.5 * (v1.x * (v3.y - v2.y) + v2.x * (v1.y - v3.y) + v3.x * (v2.y - v1.y))
}

#[inline]
pub fn vec3_to_vec2_by_plane(
    plane_class: PlaneClass,
    normal: Vector3<f32>,
    point: Vector3<f32>,
) -> Vector2<f32> {
    match plane_class {
        PlaneClass::XY => {
            if normal.z < 0.0 {
                Vector2::new(point.y, point.x)
            } else {
                Vector2::new(point.x, point.y)
            }
        }
        PlaneClass::XZ => {
            if normal.y < 0.0 {
                Vector2::new(point.x, point.z)
            } else {
                Vector2::new(point.z, point.x)
            }
        }
        PlaneClass::YZ => {
            if normal.x < 0.0 {
                Vector2::new(point.z, point.y)
            } else {
                Vector2::new(point.y, point.z)
            }
        }
    }
}

#[inline]
pub fn is_point_inside_2d_triangle(
    point: Vector2<f32>,
    pt_a: Vector2<f32>,
    pt_b: Vector2<f32>,
    pt_c: Vector2<f32>,
) -> bool {
    let ba = pt_b - pt_a;
    let ca = pt_c - pt_a;

    let vp = point - pt_a;

    let ba_dot_ba = ba.dot(&ba);
    let ca_dot_ba = ca.dot(&ba);
    let ca_dot_ca = ca.dot(&ca);

    let dot_02 = ca.dot(&vp);
    let dot_12 = ba.dot(&vp);

    let inv_denom = 1.0 / (ca_dot_ca * ba_dot_ba - ca_dot_ba.powi(2));

    // calculate barycentric coordinates
    let u = (ba_dot_ba * dot_02 - ca_dot_ba * dot_12) * inv_denom;
    let v = (ca_dot_ca * dot_12 - ca_dot_ba * dot_02) * inv_denom;

    (u >= 0.0) && (v >= 0.0) && (u + v < 1.0)
}

#[inline]
pub fn wrap_angle(angle: f32) -> f32 {
    let two_pi = 2.0 * std::f32::consts::PI;

    if angle > 0.0 {
        angle % two_pi
    } else {
        (angle + two_pi) % two_pi
    }
}

/// There are two versions of remainder, the standard `%` operator which does `x - (x/y).trunc()*y` and IEEE remainder which does `x - (x/y).round()*y`.
#[inline]
pub fn ieee_remainder(x: f32, y: f32) -> f32 {
    x - (x / y).round() * y
}

#[inline]
pub fn round_to_step(x: f32, step: f32) -> f32 {
    x - ieee_remainder(x, step)
}

#[inline]
pub fn wrapf(mut n: f32, mut min_limit: f32, mut max_limit: f32) -> f32 {
    if n >= min_limit && n <= max_limit {
        return n;
    }

    if max_limit == 0.0 && min_limit == 0.0 {
        return 0.0;
    }

    max_limit -= min_limit;

    let offset = min_limit;
    min_limit = 0.0;
    n -= offset;

    let num_of_max = (n / max_limit).abs().floor();

    if n >= max_limit {
        n -= num_of_max * max_limit;
    } else if n < min_limit {
        n += (num_of_max + 1.0) * max_limit;
    }

    n + offset
}

#[inline(always)]
pub fn lerpf(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// https://en.wikipedia.org/wiki/Cubic_Hermite_spline
#[inline]
pub fn cubicf(p0: f32, p1: f32, t: f32, m0: f32, m1: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let scale = (p1 - p0).abs();

    (2.0 * t3 - 3.0 * t2 + 1.0) * p0
        + (t3 - 2.0 * t2 + t) * m0 * scale
        + (-2.0 * t3 + 3.0 * t2) * p1
        + (t3 - t2) * m1 * scale
}

#[inline]
pub fn cubicf_derivative(p0: f32, p1: f32, t: f32, m0: f32, m1: f32) -> f32 {
    let t2 = t * t;
    let scale = (p1 - p0).abs();

    (6.0 * t2 - 6.0 * t) * p0
        + (3.0 * t2 - 4.0 * t + 1.0) * m0 * scale
        + (6.0 * t - 6.0 * t2) * p1
        + (3.0 * t2 - 2.0 * t) * m1 * scale
}

#[inline]
pub fn inf_sup_cubicf(p0: f32, p1: f32, m0: f32, m1: f32) -> (f32, f32) {
    // Find two `t`s where derivative of cubicf is zero - these will be
    // extreme points of the spline. Then get the values at those `t`s
    let d = -(9.0 * p0 * p0 + 6.0 * p0 * (-3.0 * p1 + m1 + m0) + 9.0 * p1 * p1
        - 6.0 * p1 * (m1 + m0)
        + m1 * m1
        + m1 * m0
        + m0 * m0)
        .sqrt();
    let k = 3.0 * (2.0 * p0 - 2.0 * p1 + m1 + m0);
    let v = 3.0 * p0 - 3.0 * p1 + m1 + 2.0 * m0;
    let t0 = (-d + v) / k;
    let t1 = (d + v) / k;
    (cubicf(p0, p1, t0, m0, m1), cubicf(p0, p1, t1, m0, m1))
}

#[inline]
pub fn get_farthest_point(points: &[Vector3<f32>], dir: Vector3<f32>) -> Vector3<f32> {
    let mut n_farthest = 0;
    let mut max_dot = -f32::MAX;
    for (i, point) in points.iter().enumerate() {
        let dot = dir.dot(point);
        if dot > max_dot {
            n_farthest = i;
            max_dot = dot
        }
    }
    points[n_farthest]
}

#[inline]
pub fn get_barycentric_coords(
    p: &Vector3<f32>,
    a: &Vector3<f32>,
    b: &Vector3<f32>,
    c: &Vector3<f32>,
) -> (f32, f32, f32) {
    let v0 = *b - *a;
    let v1 = *c - *a;
    let v2 = *p - *a;

    let d00 = v0.dot(&v0);
    let d01 = v0.dot(&v1);
    let d11 = v1.dot(&v1);
    let d20 = v2.dot(&v0);
    let d21 = v2.dot(&v1);
    let denom = d00 * d11 - d01.powi(2);

    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    let u = 1.0 - v - w;

    (u, v, w)
}

#[inline]
pub fn get_barycentric_coords_2d(
    p: Vector2<f32>,
    a: Vector2<f32>,
    b: Vector2<f32>,
    c: Vector2<f32>,
) -> (f32, f32, f32) {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;

    let d00 = v0.dot(&v0);
    let d01 = v0.dot(&v1);
    let d11 = v1.dot(&v1);
    let d20 = v2.dot(&v0);
    let d21 = v2.dot(&v1);
    let inv_denom = 1.0 / (d00 * d11 - d01.powi(2));

    let v = (d11 * d20 - d01 * d21) * inv_denom;
    let w = (d00 * d21 - d01 * d20) * inv_denom;
    let u = 1.0 - v - w;

    (u, v, w)
}

#[inline]
pub fn barycentric_to_world(
    bary: (f32, f32, f32),
    pa: Vector3<f32>,
    pb: Vector3<f32>,
    pc: Vector3<f32>,
) -> Vector3<f32> {
    pa.scale(bary.0) + pb.scale(bary.1) + pc.scale(bary.2)
}

#[inline]
pub fn barycentric_is_inside(bary: (f32, f32, f32)) -> bool {
    (bary.0 >= 0.0) && (bary.1 >= 0.0) && (bary.0 + bary.1 < 1.0)
}

#[inline]
pub fn is_point_inside_triangle(p: &Vector3<f32>, vertices: &[Vector3<f32>; 3]) -> bool {
    let ba = vertices[1] - vertices[0];
    let ca = vertices[2] - vertices[0];
    let vp = *p - vertices[0];

    let ba_dot_ba = ba.dot(&ba);
    let ca_dot_ba = ca.dot(&ba);
    let ca_dot_ca = ca.dot(&ca);

    let dot02 = ca.dot(&vp);
    let dot12 = ba.dot(&vp);

    let inv_denom = 1.0 / (ca_dot_ca * ba_dot_ba - ca_dot_ba.powi(2));

    // Calculate barycentric coordinates
    let u = (ba_dot_ba * dot02 - ca_dot_ba * dot12) * inv_denom;
    let v = (ca_dot_ca * dot12 - ca_dot_ba * dot02) * inv_denom;

    (u >= 0.0) && (v >= 0.0) && (u + v < 1.0)
}

#[inline]
pub fn triangle_area(a: Vector3<f32>, b: Vector3<f32>, c: Vector3<f32>) -> f32 {
    (b - a).cross(&(c - a)).norm() * 0.5
}

#[inline]
pub fn solve_quadratic(a: f32, b: f32, c: f32) -> Option<[f32; 2]> {
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        // No real roots
        None
    } else {
        // Dont care if quadratic equation has only one root (discriminant == 0), this is edge-case
        // which requires additional branching instructions which is not good for branch-predictor in CPU.
        let _2a = 2.0 * a;
        let discr_root = discriminant.sqrt();
        let r1 = (-b + discr_root) / _2a;
        let r2 = (-b - discr_root) / _2a;
        Some([r1, r2])
    }
}

#[inline]
pub fn spherical_to_cartesian(azimuth: f32, elevation: f32, radius: f32) -> Vector3<f32> {
    let x = radius * elevation.sin() * azimuth.sin();
    let y = radius * elevation.cos();
    let z = -radius * elevation.sin() * azimuth.cos();
    Vector3::new(x, y, z)
}

#[inline]
pub fn ray_rect_intersection(
    rect: Rect<f32>,
    origin: Vector2<f32>,
    dir: Vector2<f32>,
) -> Option<IntersectionResult> {
    let min = rect.left_top_corner();
    let max = rect.right_bottom_corner();

    let (mut tmin, mut tmax) = if dir.x >= 0.0 {
        ((min.x - origin.x) / dir.x, (max.x - origin.x) / dir.x)
    } else {
        ((max.x - origin.x) / dir.x, (min.x - origin.x) / dir.x)
    };

    let (tymin, tymax) = if dir.y >= 0.0 {
        ((min.y - origin.y) / dir.y, (max.y - origin.y) / dir.y)
    } else {
        ((max.y - origin.y) / dir.y, (min.y - origin.y) / dir.y)
    };

    if tmin > tymax || tymin > tmax {
        return None;
    }
    if tymin > tmin {
        tmin = tymin;
    }
    if tymax < tmax {
        tmax = tymax;
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

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TriangleEdge {
    pub a: u32,
    pub b: u32,
}

impl PartialEq for TriangleEdge {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b || self.a == other.b && self.b == other.a
    }
}

impl Eq for TriangleEdge {}

impl Hash for TriangleEdge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Direction-agnostic hash.
        (self.a as u64 + self.b as u64).hash(state)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct TriangleDefinition(pub [u32; 3]);

impl TriangleDefinition {
    #[inline]
    pub fn indices(&self) -> &[u32] {
        self.as_ref()
    }

    #[inline]
    pub fn indices_mut(&mut self) -> &mut [u32] {
        self.as_mut()
    }

    #[inline]
    pub fn edges(&self) -> [TriangleEdge; 3] {
        [
            TriangleEdge {
                a: self.0[0],
                b: self.0[1],
            },
            TriangleEdge {
                a: self.0[1],
                b: self.0[2],
            },
            TriangleEdge {
                a: self.0[2],
                b: self.0[0],
            },
        ]
    }

    #[inline]
    pub fn add(&self, i: u32) -> Self {
        Self([self.0[0] + i, self.0[1] + i, self.0[2] + i])
    }
}

impl Index<usize> for TriangleDefinition {
    type Output = u32;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for TriangleDefinition {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

pub trait PositionProvider: Sized {
    fn position(&self) -> Vector3<f32>;
}

impl PositionProvider for Vector3<f32> {
    #[inline]
    fn position(&self) -> Vector3<f32> {
        *self
    }
}

impl AsRef<[u32]> for TriangleDefinition {
    #[inline]
    fn as_ref(&self) -> &[u32] {
        &self.0
    }
}

impl AsMut<[u32]> for TriangleDefinition {
    #[inline]
    fn as_mut(&mut self) -> &mut [u32] {
        &mut self.0
    }
}

/// Tries to find a point closest to given point.
///
/// # Notes
///
/// O(n) complexity.
#[inline]
pub fn get_closest_point<P: PositionProvider>(points: &[P], point: Vector3<f32>) -> Option<usize> {
    let mut closest_sqr_distance = f32::MAX;
    let mut closest_index = None;
    for (i, vertex) in points.iter().enumerate() {
        let sqr_distance = (vertex.position() - point).norm_squared();
        if sqr_distance < closest_sqr_distance {
            closest_sqr_distance = sqr_distance;
            closest_index = Some(i);
        }
    }
    closest_index
}

/// Returns a tuple of (point index; triangle index) closest to the given point.
#[inline]
pub fn get_closest_point_triangles<P>(
    points: &[P],
    triangles: &[TriangleDefinition],
    triangle_indices: impl Iterator<Item = usize>,
    point: Vector3<f32>,
) -> Option<(usize, usize)>
where
    P: PositionProvider,
{
    let mut closest_sqr_distance = f32::MAX;
    let mut closest_index = None;
    for triangle_index in triangle_indices {
        let triangle = triangles.get(triangle_index).unwrap();
        for point_index in triangle.0.iter() {
            let vertex = points.get(*point_index as usize).unwrap();
            let sqr_distance = (vertex.position() - point).norm_squared();
            if sqr_distance < closest_sqr_distance {
                closest_sqr_distance = sqr_distance;
                closest_index = Some((*point_index as usize, triangle_index));
            }
        }
    }
    closest_index
}

#[inline]
pub fn get_arbitrary_line_perpendicular(
    begin: Vector3<f32>,
    end: Vector3<f32>,
) -> Option<Vector3<f32>> {
    let dir = (end - begin).try_normalize(f32::EPSILON)?;
    for axis in [Vector3::z(), Vector3::y(), Vector3::x()] {
        let perp = dir.cross(&axis);
        if perp.norm_squared().ne(&0.0) {
            return Some(perp);
        }
    }
    None
}

/// Returns a tuple of (point index; triangle index) closest to the given point.
#[inline]
pub fn get_closest_point_triangle_set<P>(
    points: &[P],
    triangles: &[TriangleDefinition],
    point: Vector3<f32>,
) -> Option<(usize, usize)>
where
    P: PositionProvider,
{
    let mut closest_sqr_distance = f32::MAX;
    let mut closest_index = None;
    for (triangle_index, triangle) in triangles.iter().enumerate() {
        for point_index in triangle.0.iter() {
            let vertex = points.get(*point_index as usize).unwrap();
            let sqr_distance = (vertex.position() - point).norm_squared();
            if sqr_distance < closest_sqr_distance {
                closest_sqr_distance = sqr_distance;
                closest_index = Some((*point_index as usize, triangle_index));
            }
        }
    }
    closest_index
}

#[derive(Debug, PartialEq, Clone)]
pub struct SmoothAngle {
    /// Current angle in radians.
    pub angle: f32,

    /// Target angle in radians.
    pub target: f32,

    /// Turn speed in radians per second (rad/s)
    pub speed: f32,
}

impl SmoothAngle {
    #[inline]
    pub fn set_target(&mut self, angle: f32) -> &mut Self {
        self.target = angle;
        self
    }

    #[inline]
    pub fn update(&mut self, dt: f32) -> &mut Self {
        self.target = wrap_angle(self.target);
        self.angle = wrap_angle(self.angle);
        if !self.at_target() {
            let delta = self.speed * dt;
            if self.distance().abs() > delta {
                self.angle += self.turn_direction() * delta;
            } else {
                self.angle = self.target;
            }
        }
        self
    }

    #[inline]
    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.speed = speed;
        self
    }

    #[inline]
    pub fn set_angle(&mut self, angle: f32) -> &mut Self {
        self.angle = angle;
        self
    }

    #[inline]
    pub fn angle(&self) -> f32 {
        self.angle
    }

    #[inline]
    pub fn at_target(&self) -> bool {
        (self.target - self.angle).abs() <= f32::EPSILON
    }

    #[inline]
    pub fn distance(&self) -> f32 {
        let diff = (self.target - self.angle + std::f32::consts::PI) % std::f32::consts::TAU
            - std::f32::consts::PI;
        if diff < -std::f32::consts::PI {
            diff + std::f32::consts::TAU
        } else {
            diff
        }
    }

    #[inline]
    fn turn_direction(&self) -> f32 {
        let distance = self.distance();

        if distance < 0.0 {
            if distance < -std::f32::consts::PI {
                1.0
            } else {
                -1.0
            }
        } else if distance > std::f32::consts::PI {
            -1.0
        } else {
            1.0
        }
    }
}

impl Default for SmoothAngle {
    fn default() -> Self {
        Self {
            angle: 0.0,
            target: 0.0,
            speed: 1.0,
        }
    }
}

#[derive(Copy, Clone, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub enum RotationOrder {
    XYZ,
    XZY,
    YZX,
    YXZ,
    ZXY,
    ZYX,
}

#[inline]
pub fn quat_from_euler<T: SimdRealField + RealField + Copy + Clone>(
    euler_radians: Vector3<T>,
    order: RotationOrder,
) -> UnitQuaternion<T> {
    let qx = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), euler_radians.x);
    let qy = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), euler_radians.y);
    let qz = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), euler_radians.z);
    match order {
        RotationOrder::XYZ => qz * qy * qx,
        RotationOrder::XZY => qy * qz * qx,
        RotationOrder::YZX => qx * qz * qy,
        RotationOrder::YXZ => qz * qx * qy,
        RotationOrder::ZXY => qy * qx * qz,
        RotationOrder::ZYX => qx * qy * qz,
    }
}

pub trait Matrix4Ext<T: Scalar> {
    fn side(&self) -> Vector3<T>;
    fn up(&self) -> Vector3<T>;
    fn look(&self) -> Vector3<T>;
    fn position(&self) -> Vector3<T>;
    fn basis(&self) -> Matrix3<T>;
}

impl<T: Scalar + Default + Copy + Clone> Matrix4Ext<T> for Matrix4<T> {
    #[inline]
    fn side(&self) -> Vector3<T> {
        Vector3::new(self[0], self[1], self[2])
    }

    #[inline]
    fn up(&self) -> Vector3<T> {
        Vector3::new(self[4], self[5], self[6])
    }

    #[inline]
    fn look(&self) -> Vector3<T> {
        Vector3::new(self[8], self[9], self[10])
    }

    #[inline]
    fn position(&self) -> Vector3<T> {
        Vector3::new(self[12], self[13], self[14])
    }

    #[inline]
    fn basis(&self) -> Matrix3<T> {
        self.fixed_resize::<3, 3>(T::default())
    }
}

pub trait Matrix3Ext<T: Scalar> {
    fn side(&self) -> Vector3<T>;
    fn up(&self) -> Vector3<T>;
    fn look(&self) -> Vector3<T>;
}

impl<T: Scalar + Copy + Clone> Matrix3Ext<T> for Matrix3<T> {
    #[inline]
    fn side(&self) -> Vector3<T> {
        Vector3::new(self[0], self[1], self[2])
    }

    #[inline]
    fn up(&self) -> Vector3<T> {
        Vector3::new(self[3], self[4], self[5])
    }

    #[inline]
    fn look(&self) -> Vector3<T> {
        Vector3::new(self[6], self[7], self[8])
    }
}

pub trait Vector3Ext {
    fn follow(&mut self, other: &Self, fraction: f32);

    fn sqr_distance(&self, other: &Self) -> f32;

    fn non_uniform_scale(&self, other: &Self) -> Self;
}

impl Vector3Ext for Vector3<f32> {
    #[inline]
    fn follow(&mut self, other: &Self, fraction: f32) {
        self.x += (other.x - self.x) * fraction;
        self.y += (other.y - self.y) * fraction;
        self.z += (other.z - self.z) * fraction;
    }

    #[inline]
    fn sqr_distance(&self, other: &Self) -> f32 {
        (self - other).norm_squared()
    }

    #[inline]
    fn non_uniform_scale(&self, other: &Self) -> Self {
        Self::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }
}

pub trait Vector2Ext {
    fn follow(&mut self, other: &Self, fraction: f32);

    fn per_component_min(&self, other: &Self) -> Self;
    fn per_component_max(&self, other: &Self) -> Self;
}

impl Vector2Ext for Vector2<f32> {
    #[inline]
    fn follow(&mut self, other: &Self, fraction: f32) {
        self.x += (other.x - self.x) * fraction;
        self.y += (other.y - self.y) * fraction;
    }

    #[inline]
    fn per_component_min(&self, other: &Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    #[inline]
    fn per_component_max(&self, other: &Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }
}

/// Returns rotation quaternion that represents rotation basis with Z axis aligned on `vec`.
/// This function handles singularities for you.
#[inline]
pub fn vector_to_quat(vec: Vector3<f32>) -> UnitQuaternion<f32> {
    let dot = vec.normalize().dot(&Vector3::y());

    if dot.abs() > 1.0 - 10.0 * f32::EPSILON {
        // Handle singularity when vector is collinear with Y axis.
        UnitQuaternion::from_axis_angle(&Vector3::x_axis(), -dot.signum() * 90.0f32.to_radians())
    } else {
        UnitQuaternion::face_towards(&vec, &Vector3::y())
    }
}

#[inline]
pub fn m4x4_approx_eq(a: &Matrix4<f32>, b: &Matrix4<f32>) -> bool {
    a.iter()
        .zip(b.iter())
        .all(|(a, b)| (*a - *b).abs() <= 0.001)
}

#[cfg(test)]
mod test {
    use nalgebra::{Matrix3, Matrix4, UnitQuaternion, Vector3};
    use num_traits::Zero;

    use super::{
        barycentric_is_inside, barycentric_to_world, cubicf_derivative, get_barycentric_coords,
        get_barycentric_coords_2d, get_closest_point, get_closest_point_triangle_set,
        get_closest_point_triangles, get_farthest_point, get_signed_triangle_area, ieee_remainder,
        inf_sup_cubicf, quat_from_euler, round_to_step, spherical_to_cartesian, triangle_area,
        wrap_angle, wrapf, Matrix3Ext, Matrix4Ext, PositionProvider, Rect, RotationOrder,
        SmoothAngle, TriangleDefinition, TriangleEdge, Vector2Ext, Vector3Ext,
    };
    use nalgebra::Vector2;

    #[test]
    fn ray_rect_intersection() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);

        // Edge-case: Horizontal ray.
        assert!(super::ray_rect_intersection(
            rect,
            Vector2::new(-1.0, 5.0),
            Vector2::new(1.0, 0.0)
        )
        .is_some());

        // Edge-case: Vertical ray.
        assert!(super::ray_rect_intersection(
            rect,
            Vector2::new(5.0, -1.0),
            Vector2::new(0.0, 1.0)
        )
        .is_some());
    }

    #[test]
    fn smooth_angle() {
        let mut angle = SmoothAngle {
            angle: 290.0f32.to_radians(),
            target: 90.0f32.to_radians(),
            speed: 100.0f32.to_radians(),
        };

        while !angle.at_target() {
            println!("{}", angle.update(1.0).angle().to_degrees());
        }
    }

    #[test]
    fn default_for_rect() {
        assert_eq!(
            Rect::<f32>::default(),
            Rect {
                position: Vector2::new(Zero::zero(), Zero::zero()),
                size: Vector2::new(Zero::zero(), Zero::zero()),
            }
        );
    }

    #[test]
    fn rect_with_position() {
        let rect = Rect::new(0, 0, 1, 1);

        assert_eq!(
            rect.with_position(Vector2::new(1, 1)),
            Rect::new(1, 1, 1, 1)
        );
    }

    #[test]
    fn rect_with_size() {
        let rect = Rect::new(0, 0, 1, 1);

        assert_eq!(
            rect.with_size(Vector2::new(10, 10)),
            Rect::new(0, 0, 10, 10)
        );
    }

    #[test]
    fn rect_inflate() {
        let rect = Rect::new(0, 0, 1, 1);

        assert_eq!(rect.inflate(5, 5), Rect::new(-5, -5, 11, 11));
    }

    #[test]
    fn rect_deflate() {
        let rect = Rect::new(-5, -5, 11, 11);

        assert_eq!(rect.deflate(5, 5), Rect::new(0, 0, 1, 1));
    }

    #[test]
    fn rect_contains() {
        let rect = Rect::new(0, 0, 10, 10);

        assert!(rect.contains(Vector2::new(0, 0)));
        assert!(rect.contains(Vector2::new(0, 10)));
        assert!(rect.contains(Vector2::new(10, 0)));
        assert!(rect.contains(Vector2::new(10, 10)));
        assert!(rect.contains(Vector2::new(5, 5)));

        assert!(!rect.contains(Vector2::new(0, 20)));
    }

    #[test]
    fn rect_center() {
        let rect = Rect::new(0, 0, 10, 10);

        assert_eq!(rect.center(), Vector2::new(5, 5));
    }

    #[test]
    fn rect_push() {
        let mut rect = Rect::new(10, 10, 11, 11);

        rect.push(Vector2::new(0, 0));
        assert_eq!(rect, Rect::new(0, 0, 21, 21));

        rect.push(Vector2::new(0, 20));
        assert_eq!(rect, Rect::new(0, 0, 21, 21));

        rect.push(Vector2::new(20, 20));
        assert_eq!(rect, Rect::new(0, 0, 21, 21));

        rect.push(Vector2::new(30, 30));
        assert_eq!(rect, Rect::new(0, 0, 30, 30));
    }

    #[test]
    fn rect_getters() {
        let rect = Rect::new(0, 0, 1, 1);

        assert_eq!(rect.left_top_corner(), Vector2::new(0, 0));
        assert_eq!(rect.left_bottom_corner(), Vector2::new(0, 1));
        assert_eq!(rect.right_top_corner(), Vector2::new(1, 0));
        assert_eq!(rect.right_bottom_corner(), Vector2::new(1, 1));

        assert_eq!(rect.x(), 0);
        assert_eq!(rect.y(), 0);
        assert_eq!(rect.w(), 1);
        assert_eq!(rect.h(), 1);
    }

    #[test]
    fn rect_clip_by() {
        let rect = Rect::new(0, 0, 10, 10);

        assert_eq!(
            rect.clip_by(Rect::new(2, 2, 1, 1)).unwrap(),
            Rect::new(2, 2, 1, 1)
        );
        assert_eq!(
            rect.clip_by(Rect::new(0, 0, 15, 15)).unwrap(),
            Rect::new(0, 0, 10, 10)
        );

        // When there is no intersection.
        assert!(rect.clip_by(Rect::new(-2, 1, 1, 1)).is_none());
        assert!(rect.clip_by(Rect::new(11, 1, 1, 1)).is_none());
        assert!(rect.clip_by(Rect::new(1, -2, 1, 1)).is_none());
        assert!(rect.clip_by(Rect::new(1, 11, 1, 1)).is_none());
    }

    #[test]
    fn rect_translate() {
        let rect = Rect::new(0, 0, 10, 10);

        assert_eq!(rect.translate(Vector2::new(5, 5)), Rect::new(5, 5, 10, 10));
    }

    #[test]
    fn rect_intersects_circle() {
        let rect = Rect::new(0.0, 0.0, 1.0, 1.0);

        assert!(!rect.intersects_circle(Vector2::new(5.0, 5.0), 1.0));
        assert!(rect.intersects_circle(Vector2::new(0.0, 0.0), 1.0));
        assert!(rect.intersects_circle(Vector2::new(-0.5, -0.5), 1.0));
    }

    #[test]
    fn rect_extend_to_contain() {
        let mut rect = Rect::new(0.0, 0.0, 1.0, 1.0);

        rect.extend_to_contain(Rect::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(rect, Rect::new(0.0, 0.0, 2.0, 2.0));

        rect.extend_to_contain(Rect::new(-1.0, -1.0, 1.0, 1.0));
        assert_eq!(rect, Rect::new(-1.0, -1.0, 3.0, 3.0));
    }

    #[test]
    fn rect_transform() {
        let rect = Rect::new(0.0, 0.0, 1.0, 1.0);

        assert_eq!(
            rect.transform(&Matrix3::new(
                1.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, //
                0.0, 0.0, 1.0,
            )),
            rect,
        );

        assert_eq!(
            rect.transform(&Matrix3::new(
                2.0, 0.0, 0.0, //
                0.0, 2.0, 0.0, //
                0.0, 0.0, 2.0,
            )),
            Rect::new(0.0, 0.0, 2.0, 2.0),
        );
    }

    #[test]
    fn test_get_signed_triangle_area() {
        assert_eq!(
            get_signed_triangle_area(
                Vector2::new(0.0, 0.0),
                Vector2::new(0.0, 1.0),
                Vector2::new(1.0, 0.0)
            ),
            0.5
        );
        assert_eq!(
            get_signed_triangle_area(
                Vector2::new(1.0, 1.0),
                Vector2::new(0.0, 1.0),
                Vector2::new(1.0, 0.0)
            ),
            -0.5
        );
    }

    #[test]
    fn test_wrap_angle() {
        let angle = 0.5 * std::f32::consts::PI;
        assert_eq!(wrap_angle(angle), angle);
        assert_eq!(wrap_angle(-angle), 3.0 * angle);
    }

    #[test]
    fn test_ieee_remainder() {
        assert_eq!(ieee_remainder(1.0, 2.0), -1.0);
        assert_eq!(ieee_remainder(3.0, 2.0), -1.0);

        assert_eq!(ieee_remainder(1.0, 3.0), 1.0);
        assert_eq!(ieee_remainder(4.0, 3.0), 1.0);

        assert_eq!(ieee_remainder(-1.0, 2.0), 1.0);
        assert_eq!(ieee_remainder(-3.0, 2.0), 1.0);
    }

    #[test]
    fn test_round_to_step() {
        assert_eq!(round_to_step(1.0, 2.0), 2.0);
        assert_eq!(round_to_step(3.0, 2.0), 4.0);

        assert_eq!(round_to_step(-1.0, 2.0), -2.0);
        assert_eq!(round_to_step(-3.0, 2.0), -4.0);
    }

    #[test]
    fn test_wrapf() {
        assert_eq!(wrapf(5.0, 0.0, 10.0), 5.0);
        assert_eq!(wrapf(5.0, 0.0, 0.0), 0.0);
        assert_eq!(wrapf(2.0, 5.0, 10.0), 7.0);
        assert_eq!(wrapf(12.0, 5.0, 10.0), 7.0);
    }

    #[test]
    fn test_cubicf_derivative() {
        assert_eq!(cubicf_derivative(1.0, 1.0, 1.0, 1.0, 1.0), 0.0);
        assert_eq!(cubicf_derivative(2.0, 1.0, 1.0, 1.0, 1.0), 1.0);
    }

    #[test]
    fn test_inf_sup_cubicf() {
        assert_eq!(inf_sup_cubicf(1.0, 1.0, 1.0, 1.0), (1.0, 1.0));
        assert_eq!(inf_sup_cubicf(2.0, 2.0, 1.0, 1.0), (2.0, 2.0));
    }

    #[test]
    fn test_get_farthest_point() {
        let points = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
        ];

        assert_eq!(
            get_farthest_point(&points, Vector3::new(1.0, 0.0, 0.0)),
            Vector3::new(1.0, 0.0, 0.0)
        );
        assert_eq!(
            get_farthest_point(&points, Vector3::new(10.0, 0.0, 0.0)),
            Vector3::new(1.0, 0.0, 0.0)
        );
        assert_eq!(
            get_farthest_point(&points, Vector3::new(1.0, 1.0, 0.0)),
            Vector3::new(1.0, 1.0, 1.0)
        );
    }

    #[test]
    fn test_get_barycentric_coords() {
        assert_eq!(
            get_barycentric_coords(
                &Vector3::new(0.0, 0.0, 0.0),
                &Vector3::new(1.0, 0.0, 0.0),
                &Vector3::new(0.0, 1.0, 0.0),
                &Vector3::new(0.0, 0.0, 1.0),
            ),
            (0.33333328, 0.33333334, 0.33333334)
        );
    }

    #[test]
    fn test_get_barycentric_coords_2d() {
        assert_eq!(
            get_barycentric_coords_2d(
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(0.0, 1.0),
                Vector2::new(0.0, 0.0),
            ),
            (0.0, 0.0, 1.0)
        );
    }

    #[test]
    fn test_barycentric_to_world() {
        assert_eq!(
            barycentric_to_world(
                (2.0, 2.0, 2.0),
                Vector3::new(1.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector3::new(0.0, 0.0, 1.0),
            ),
            Vector3::new(2.0, 2.0, 2.0)
        );
    }

    #[test]
    fn test_barycentric_is_inside() {
        assert!(barycentric_is_inside((0.0, 0.0, 0.0)));
        assert!(barycentric_is_inside((0.5, 0.49, 0.0)));

        assert!(!barycentric_is_inside((0.5, 0.5, 0.0)));
        assert!(!barycentric_is_inside((-0.5, 0.49, 0.0)));
        assert!(!barycentric_is_inside((0.5, -0.49, 0.0)));
        assert!(!barycentric_is_inside((-0.5, -0.49, 0.0)));
    }

    #[test]
    fn test_triangle_area() {
        assert_eq!(
            triangle_area(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                Vector3::new(0.0, 0.0, 1.0),
            ),
            0.5
        );
    }

    #[test]
    fn test_spherical_to_cartesian() {
        assert_eq!(
            spherical_to_cartesian(0.0, 0.0, 1.0),
            Vector3::new(0.0, 1.0, 0.0)
        );
    }

    #[test]
    fn partial_eq_for_triangle_edge() {
        let te = TriangleEdge { a: 2, b: 5 };
        let te2 = TriangleEdge { a: 2, b: 5 };
        let te3 = TriangleEdge { a: 5, b: 2 };

        assert_eq!(te, te2);
        assert_eq!(te, te3);
    }

    #[test]
    fn triangle_definition_indices() {
        assert_eq!(TriangleDefinition([0, 0, 0]).indices(), &[0, 0, 0]);
    }

    #[test]
    fn triangle_definition_indices_mut() {
        assert_eq!(TriangleDefinition([0, 0, 0]).indices_mut(), &mut [0, 0, 0]);
    }

    #[test]
    fn triangle_definition_edges() {
        let t = TriangleDefinition([0, 1, 2]);
        assert_eq!(
            t.edges(),
            [
                TriangleEdge { a: 0, b: 1 },
                TriangleEdge { a: 1, b: 2 },
                TriangleEdge { a: 2, b: 0 }
            ]
        );
    }

    #[test]
    fn index_for_triangle_definition() {
        let t = TriangleDefinition([0, 1, 2]);

        assert_eq!(t[0], 0);
        assert_eq!(t[1], 1);
        assert_eq!(t[2], 2);
    }

    #[test]
    fn index_mut_for_triangle_definition() {
        let mut t = TriangleDefinition([5, 5, 5]);
        t[0] = 0;
        t[1] = 1;
        t[2] = 2;

        assert_eq!(t[0], 0);
        assert_eq!(t[1], 1);
        assert_eq!(t[2], 2);
    }

    #[test]
    fn position_provider_for_vector3() {
        let v = Vector3::new(0.0, 1.0, 2.0);

        assert_eq!(v.position(), v);
    }

    #[test]
    fn as_ref_for_triangle_definition() {
        let t = TriangleDefinition([0, 1, 2]);

        assert_eq!(t.as_ref(), &[0, 1, 2]);
    }

    #[test]
    fn as_mut_for_triangle_definition() {
        let mut t = TriangleDefinition([0, 1, 2]);

        assert_eq!(t.as_mut(), &mut [0, 1, 2]);
    }

    #[test]
    fn test_get_closest_point() {
        let points = [
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];

        assert_eq!(
            get_closest_point(&points, Vector3::new(0.0, 0.0, 0.0)),
            Some(0),
        );
        assert_eq!(
            get_closest_point(&points, Vector3::new(0.0, 1.0, 1.0)),
            Some(1),
        );
        assert_eq!(
            get_closest_point(&points, Vector3::new(0.0, 0.0, 10.0)),
            Some(2),
        );
    }

    #[test]
    fn test_get_closest_point_triangles() {
        let points = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];
        let triangles = [TriangleDefinition([0, 1, 2]), TriangleDefinition([1, 2, 3])];

        assert_eq!(
            get_closest_point_triangles(
                &points,
                &triangles,
                [0, 1].into_iter(),
                Vector3::new(1.0, 1.0, 1.0)
            ),
            Some((1, 0))
        );
    }

    #[test]
    fn test_get_closest_point_triangle_set() {
        let points = [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];
        let triangles = [TriangleDefinition([0, 1, 2]), TriangleDefinition([1, 2, 3])];

        assert_eq!(
            get_closest_point_triangle_set(&points, &triangles, Vector3::new(1.0, 1.0, 1.0)),
            Some((1, 0))
        );
    }

    #[test]
    fn smooth_angle_setters() {
        let mut sa = SmoothAngle {
            angle: 0.0,
            speed: 0.0,
            target: 0.0,
        };

        assert_eq!(sa.angle(), 0.0);

        sa.set_angle(std::f32::consts::PI);
        assert_eq!(sa.angle(), std::f32::consts::PI);

        sa.set_target(std::f32::consts::PI);
        assert_eq!(sa.target, std::f32::consts::PI);

        sa.set_speed(1.0);
        assert_eq!(sa.speed, 1.0);
    }

    #[test]
    fn smooth_angle_turn_direction() {
        assert_eq!(
            SmoothAngle {
                angle: 0.0,
                speed: 0.0,
                target: std::f32::consts::PI * 1.1,
            }
            .turn_direction(),
            -1.0
        );

        assert_eq!(
            SmoothAngle {
                angle: 0.0,
                speed: 0.0,
                target: -std::f32::consts::PI * 1.1,
            }
            .turn_direction(),
            1.0
        );

        assert_eq!(
            SmoothAngle {
                angle: 0.0,
                speed: 0.0,
                target: -std::f32::consts::PI * 0.9,
            }
            .turn_direction(),
            -1.0
        );

        assert_eq!(
            SmoothAngle {
                angle: 0.0,
                speed: 0.0,
                target: std::f32::consts::PI * 0.9,
            }
            .turn_direction(),
            1.0
        );
    }

    #[test]
    fn default_for_smooth_angle() {
        let sa = SmoothAngle::default();

        assert_eq!(sa.angle, 0.0);
        assert_eq!(sa.target, 0.0);
        assert_eq!(sa.speed, 1.0);
    }

    #[test]
    fn test_quat_from_euler() {
        assert_eq!(
            quat_from_euler(
                Vector3::new(
                    std::f32::consts::PI,
                    std::f32::consts::PI,
                    std::f32::consts::PI
                ),
                RotationOrder::XYZ
            ),
            UnitQuaternion::from_euler_angles(
                std::f32::consts::PI,
                std::f32::consts::PI,
                std::f32::consts::PI
            )
        );
    }

    #[test]
    fn matrix4_ext_for_matrix4() {
        let m = Matrix4::new(
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        );

        assert_eq!(m.side(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(m.up(), Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(m.look(), Vector3::new(0.0, 0.0, 1.0));
        assert_eq!(m.position(), Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(
            m.basis(),
            Matrix3::new(
                1.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, //
                0.0, 0.0, 1.0,
            )
        );
    }

    #[test]
    fn matrix3_ext_for_matrix3() {
        let m = Matrix3::new(
            1.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, //
            0.0, 0.0, 1.0,
        );

        assert_eq!(m.side(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(m.up(), Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(m.look(), Vector3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn vector3_ext_for_vector3() {
        let mut v = Vector3::new(2.0, 2.0, 2.0);

        assert_eq!(v.sqr_distance(&Vector3::new(0.0, 1.0, 1.0)), 6.0);

        assert_eq!(
            v.non_uniform_scale(&Vector3::new(3.0, 3.0, 3.0)),
            Vector3::new(6.0, 6.0, 6.0)
        );

        v.follow(&Vector3::new(0.5, 0.5, 0.5), 2.0);
        assert_eq!(v, Vector3::new(-1.0, -1.0, -1.0));
    }

    #[test]
    fn vector2_ext_for_vector2() {
        let mut v = Vector2::new(2.0, 2.0);

        assert_eq!(
            v.per_component_min(&Vector2::new(0.0, 4.0)),
            Vector2::new(0.0, 2.0)
        );

        assert_eq!(
            v.per_component_max(&Vector2::new(0.0, 4.0)),
            Vector2::new(2.0, 4.0)
        );

        v.follow(&Vector2::new(0.5, 0.5), 2.0);
        assert_eq!(v, Vector2::new(-1.0, -1.0));
    }

    #[test]
    fn test_m4x4_approx_eq() {
        assert!(crate::m4x4_approx_eq(
            &Matrix4::new(
                1.0, 0.0, 0.0, 0.0, //
                0.0, 1.0, 0.0, 0.0, //
                0.0, 0.0, 1.0, 0.0, //
                0.0, 0.0, 0.0, 1.0,
            ),
            &Matrix4::new(
                1.0001, 0.0001, 0.0001, 0.0001, //
                0.0001, 1.0001, 0.0001, 0.0001, //
                0.0001, 0.0001, 1.0001, 0.0001, //
                0.0001, 0.0001, 0.0001, 1.0001,
            )
        ),);
    }
}
