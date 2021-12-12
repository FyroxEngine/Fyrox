// Clippy complains about normal mathematical symbols like A, B, C for quadratic equation.
#![allow(clippy::many_single_char_names)]

pub mod aabb;
pub mod frustum;
pub mod plane;
pub mod ray;
pub mod triangulator;

use crate::algebra::{RealField, SimdRealField};
use crate::{
    algebra::{Matrix3, Matrix4, Scalar, UnitQuaternion, Vector2, Vector3},
    math::ray::IntersectionResult,
    num_traits::{NumAssign, Zero},
    visitor::{Visit, VisitResult, Visitor},
};
use std::ops::{Index, IndexMut};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect<T>
where
    T: NumAssign + Scalar + PartialOrd + Copy,
{
    pub position: Vector2<T>,
    pub size: Vector2<T>,
}

impl<T> Default for Rect<T>
where
    T: NumAssign + Scalar + PartialOrd + Copy,
{
    fn default() -> Self {
        Self {
            position: Vector2::new(Zero::zero(), Zero::zero()),
            size: Vector2::new(Zero::zero(), Zero::zero()),
        }
    }
}

impl<T> Rect<T>
where
    T: NumAssign + Scalar + PartialOrd + Copy,
{
    pub fn new(x: T, y: T, w: T, h: T) -> Self {
        Self {
            position: Vector2::new(x, y),
            size: Vector2::new(w, h),
        }
    }

    #[must_use = "this method creates new instance of rect"]
    pub fn inflate(&self, dw: T, dh: T) -> Self {
        Self {
            position: Vector2::new(self.position.x - dw, self.position.y - dh),
            size: Vector2::new(self.size.x + dw + dw, self.size.y + dh + dh),
        }
    }

    #[must_use = "this method creates new instance of rect"]
    pub fn deflate(&self, dw: T, dh: T) -> Self {
        Self {
            position: Vector2::new(self.position.x + dw, self.position.y + dh),
            size: Vector2::new(self.size.x - (dw + dw), self.size.y - (dh + dh)),
        }
    }

    pub fn contains(&self, pt: Vector2<T>) -> bool {
        pt.x >= self.position.x
            && pt.x <= self.position.x + self.size.x
            && pt.y >= self.position.y
            && pt.y <= self.position.y + self.size.y
    }

    /// Extends rect to contain given point.
    ///
    /// # Notes
    ///
    /// To build bounding rectangle you should correctly initialize initial rectangle:
    ///
    /// ```
    /// # use rg3d_core::algebra::Vector2;
    /// # use rg3d_core::math::Rect;
    ///
    /// let vertices = [Vector2::new(1.0, 2.0), Vector2::new(-3.0, 5.0)];
    ///
    /// // This is important part, it must have "invalid" state to correctly
    /// // calculate bounding rect. Rect::default will give invalid result!
    /// let mut bounding_rect = Rect::new(f32::MAX, f32::MAX, 0.0, 0.0);
    ///
    /// for &v in &vertices {
    ///     bounding_rect.push(v);
    /// }
    /// ```
    pub fn push(&mut self, p: Vector2<T>) {
        if p.x < self.position.x {
            self.position.x = p.x;
        }
        if p.y < self.position.y {
            self.position.y = p.y;
        }

        let right_bottom = self.right_bottom_corner();

        if p.x > right_bottom.x {
            self.size.x = p.x - self.position.x;
        }
        if p.y > right_bottom.y {
            self.size.y = p.y - self.position.y;
        }
    }

    #[must_use = "this method creates new instance of rect"]
    pub fn clip_by(&self, other: Rect<T>) -> Rect<T> {
        let mut clipped = *self;

        if clipped.position.x < other.position.x {
            clipped.position.x = other.position.x;
            clipped.size.x -= other.position.x - clipped.position.x;
        }
        if clipped.position.y < other.position.y {
            clipped.position.y = other.position.y;
            clipped.size.y -= other.position.y - clipped.position.y;
        }

        let clipped_right_bottom = clipped.right_bottom_corner();
        let other_right_bottom = other.right_bottom_corner();

        if clipped_right_bottom.x > other_right_bottom.x {
            clipped.size.x -= clipped_right_bottom.x - other_right_bottom.x;
        }
        if clipped_right_bottom.y > other_right_bottom.y {
            clipped.size.y -= clipped_right_bottom.y - other_right_bottom.y;
        }

        clipped
    }

    pub fn intersects(&self, other: Rect<T>) -> bool {
        if other.position.x < self.position.x + self.size.x
            && self.position.x < other.position.x + other.size.x
            && other.position.y < self.position.y + self.size.y
        {
            self.position.y < other.position.y + other.size.y
        } else {
            false
        }
    }

    #[must_use = "this method creates new instance of rect"]
    pub fn translate(&self, translation: Vector2<T>) -> Self {
        Self {
            position: Vector2::new(
                self.position.x + translation.x,
                self.position.y + translation.y,
            ),
            size: self.size,
        }
    }

    pub fn intersects_circle(&self, circle: Vector2<T>, radius: T) -> bool {
        let r = self.position.x + self.size.x;
        let b = self.position.y + self.size.y;
        // find the closest point to the circle within the rectangle
        let closest_x = if circle.x < self.position.x {
            self.position.x
        } else if circle.x > r {
            r
        } else {
            circle.x
        };
        let closest_y = if circle.y < self.position.y {
            self.position.y
        } else if circle.y > b {
            b
        } else {
            circle.y
        };
        // calculate the distance between the circle's center and this closest point
        let distance_x = circle.x - closest_x;
        let distance_y = circle.y - closest_y;
        // if the distance is less than the circle's radius, an intersection occurs
        let distance_squared = (distance_x * distance_x) + (distance_y * distance_y);
        distance_squared < (radius * radius)
    }

    pub fn extend_to_contain(&mut self, other: Rect<T>) {
        if other.position.x < self.position.x {
            self.position.x = other.position.x;
        }
        if other.position.y < self.position.y {
            self.position.y = other.position.y;
        }
        let self_right_bottom_corner = self.right_bottom_corner();
        let other_right_bottom_corner = other.right_bottom_corner();
        if other_right_bottom_corner.x > self_right_bottom_corner.x {
            self.size.x += other_right_bottom_corner.x - self_right_bottom_corner.x;
        }
        if other_right_bottom_corner.y > self_right_bottom_corner.y {
            self.size.y += other_right_bottom_corner.y - self_right_bottom_corner.y;
        }
    }

    #[inline(always)]
    pub fn left_top_corner(&self) -> Vector2<T> {
        self.position
    }

    #[inline(always)]
    pub fn right_top_corner(&self) -> Vector2<T> {
        Vector2::new(self.position.x + self.size.x, self.position.y)
    }

    #[inline(always)]
    pub fn right_bottom_corner(&self) -> Vector2<T> {
        Vector2::new(self.position.x + self.size.x, self.position.y + self.size.y)
    }

    #[inline(always)]
    pub fn left_bottom_corner(&self) -> Vector2<T> {
        Vector2::new(self.position.x, self.position.y + self.size.y)
    }

    #[inline(always)]
    pub fn w(&self) -> T {
        self.size.x
    }

    #[inline(always)]
    pub fn h(&self) -> T {
        self.size.y
    }

    #[inline(always)]
    pub fn x(&self) -> T {
        self.position.x
    }

    #[inline(always)]
    pub fn y(&self) -> T {
        self.position.y
    }
}

impl<T> Visit for Rect<T>
where
    T: NumAssign + Scalar + Visit + PartialOrd + Copy + 'static,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.x.visit("X", visitor)?;
        self.position.y.visit("Y", visitor)?;
        self.size.x.visit("W", visitor)?;
        self.size.y.visit("H", visitor)?;

        visitor.leave_region()
    }
}

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

#[inline]
pub fn clampf(v: f32, min: f32, max: f32) -> f32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
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

#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct TriangleEdge {
    a: u32,
    b: u32,
}

impl PartialEq for TriangleEdge {
    fn eq(&self, other: &Self) -> bool {
        self.a == other.a && self.b == other.b || self.a == other.b && self.b == other.a
    }
}

impl Eq for TriangleEdge {}

#[derive(Clone, PartialEq, Eq, Debug, Default, Hash)]
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
}

impl Visit for TriangleDefinition {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.0[0].visit("A", visitor)?;
        self.0[1].visit("B", visitor)?;
        self.0[2].visit("C", visitor)?;

        visitor.leave_region()
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

#[inline]
pub fn get_closest_point_triangles<P: PositionProvider>(
    points: &[P],
    triangles: &[TriangleDefinition],
    triangle_indices: &[u32],
    point: Vector3<f32>,
) -> Option<usize> {
    let mut closest_sqr_distance = f32::MAX;
    let mut closest_index = None;
    for triangle_index in triangle_indices {
        let triangle = triangles.get(*triangle_index as usize).unwrap();
        for point_index in triangle.0.iter() {
            let vertex = points.get(*point_index as usize).unwrap();
            let sqr_distance = (vertex.position() - point).norm_squared();
            if sqr_distance < closest_sqr_distance {
                closest_sqr_distance = sqr_distance;
                closest_index = Some(*point_index as usize);
            }
        }
    }
    closest_index
}

#[inline]
pub fn get_closest_point_triangle_set<P: PositionProvider>(
    points: &[P],
    triangles: &[TriangleDefinition],
    point: Vector3<f32>,
) -> Option<usize> {
    let mut closest_sqr_distance = f32::MAX;
    let mut closest_index = None;
    for triangle in triangles {
        for point_index in triangle.0.iter() {
            let vertex = points.get(*point_index as usize).unwrap();
            let sqr_distance = (vertex.position() - point).norm_squared();
            if sqr_distance < closest_sqr_distance {
                closest_sqr_distance = sqr_distance;
                closest_index = Some(*point_index as usize);
            }
        }
    }
    closest_index
}

#[derive(Debug, PartialEq)]
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

impl Visit for SmoothAngle {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.angle.visit("Angle", visitor)?;
        self.target.visit("Target", visitor)?;
        self.speed.visit("Speed", visitor)?;

        visitor.leave_region()
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
    fn side(&self) -> Vector3<T> {
        Vector3::new(self[0], self[1], self[2])
    }

    fn up(&self) -> Vector3<T> {
        Vector3::new(self[4], self[5], self[6])
    }

    fn look(&self) -> Vector3<T> {
        Vector3::new(self[8], self[9], self[10])
    }

    fn position(&self) -> Vector3<T> {
        Vector3::new(self[12], self[13], self[14])
    }

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
    fn side(&self) -> Vector3<T> {
        Vector3::new(self[0], self[1], self[2])
    }

    fn up(&self) -> Vector3<T> {
        Vector3::new(self[3], self[4], self[5])
    }

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

#[cfg(test)]
mod test {
    use crate::algebra::Vector2;
    use crate::math::Rect;
    use crate::math::SmoothAngle;

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
}
