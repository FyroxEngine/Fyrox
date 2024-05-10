use crate::Matrix4Ext;
use nalgebra::{Matrix4, Vector3};

#[derive(Copy, Clone, Debug)]
pub struct AxisAlignedBoundingBox {
    pub min: Vector3<f32>,
    pub max: Vector3<f32>,
}

impl Default for AxisAlignedBoundingBox {
    #[inline]
    fn default() -> Self {
        Self {
            min: Vector3::new(f32::MAX, f32::MAX, f32::MAX),
            max: Vector3::new(-f32::MAX, -f32::MAX, -f32::MAX),
        }
    }
}

impl AxisAlignedBoundingBox {
    #[inline]
    pub const fn unit() -> Self {
        Self::from_min_max(Vector3::new(-0.5, -0.5, -0.5), Vector3::new(0.5, 0.5, 0.5))
    }

    #[inline]
    pub const fn collapsed() -> Self {
        Self {
            min: Vector3::new(0.0, 0.0, 0.0),
            max: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    #[inline]
    pub fn from_radius(radius: f32) -> Self {
        Self {
            min: Vector3::new(-radius, -radius, -radius),
            max: Vector3::new(radius, radius, radius),
        }
    }

    #[inline]
    pub const fn from_min_max(min: Vector3<f32>, max: Vector3<f32>) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn from_point(point: Vector3<f32>) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    #[inline]
    pub fn from_points(points: &[Vector3<f32>]) -> Self {
        let mut aabb = AxisAlignedBoundingBox::default();
        for pt in points {
            aabb.add_point(*pt);
        }
        aabb
    }

    #[inline]
    pub fn add_point(&mut self, a: Vector3<f32>) {
        if a.x < self.min.x {
            self.min.x = a.x;
        }
        if a.y < self.min.y {
            self.min.y = a.y;
        }
        if a.z < self.min.z {
            self.min.z = a.z;
        }

        if a.x > self.max.x {
            self.max.x = a.x;
        }
        if a.y > self.max.y {
            self.max.y = a.y;
        }
        if a.z > self.max.z {
            self.max.z = a.z;
        }
    }

    #[inline]
    pub fn inflate(&mut self, delta: Vector3<f32>) {
        self.min -= delta.scale(0.5);
        self.max += delta.scale(0.5);
    }

    #[inline]
    pub fn add_box(&mut self, other: Self) {
        self.add_point(other.min);
        self.add_point(other.max);
    }

    #[inline]
    pub fn corners(&self) -> [Vector3<f32>; 8] {
        [
            Vector3::new(self.min.x, self.min.y, self.min.z),
            Vector3::new(self.min.x, self.min.y, self.max.z),
            Vector3::new(self.max.x, self.min.y, self.max.z),
            Vector3::new(self.max.x, self.min.y, self.min.z),
            Vector3::new(self.min.x, self.max.y, self.min.z),
            Vector3::new(self.min.x, self.max.y, self.max.z),
            Vector3::new(self.max.x, self.max.y, self.max.z),
            Vector3::new(self.max.x, self.max.y, self.min.z),
        ]
    }

    #[inline]
    pub fn volume(&self) -> f32 {
        let size = self.max - self.min;
        size.x * size.y * size.z
    }

    #[inline]
    pub fn offset(&mut self, v: Vector3<f32>) {
        self.min += v;
        self.max += v;
    }

    #[inline]
    pub fn center(&self) -> Vector3<f32> {
        (self.max + self.min).scale(0.5)
    }

    #[inline]
    pub fn half_extents(&self) -> Vector3<f32> {
        (self.max - self.min).scale(0.5)
    }

    #[inline]
    pub fn invalidate(&mut self) {
        *self = Default::default();
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        #[inline(always)]
        fn is_nan_or_inf(x: &Vector3<f32>) -> bool {
            x.iter().all(|e| e.is_nan() || e.is_infinite())
        }

        self.max.x >= self.min.x
            && self.max.y >= self.min.y
            && self.max.z >= self.min.z
            && !is_nan_or_inf(&self.min)
            && !is_nan_or_inf(&self.max)
    }

    #[inline]
    pub fn is_degenerate(&self) -> bool {
        self.max == self.min
    }

    #[inline]
    pub fn is_invalid_or_degenerate(&self) -> bool {
        !self.is_valid() || self.is_degenerate()
    }

    #[inline]
    pub fn is_contains_point(&self, point: Vector3<f32>) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    #[inline]
    pub fn is_intersects_sphere(&self, position: Vector3<f32>, radius: f32) -> bool {
        let r2 = radius.powi(2);
        let mut dmin = 0.0;

        if position.x < self.min.x {
            dmin += (position.x - self.min.x).powi(2);
        } else if position.x > self.max.x {
            dmin += (position.x - self.max.x).powi(2);
        }

        if position.y < self.min.y {
            dmin += (position.y - self.min.y).powi(2);
        } else if position.y > self.max.y {
            dmin += (position.y - self.max.y).powi(2);
        }

        if position.z < self.min.z {
            dmin += (position.z - self.min.z).powi(2);
        } else if position.z > self.max.z {
            dmin += (position.z - self.max.z).powi(2);
        }

        dmin <= r2
            || ((position.x >= self.min.x)
                && (position.x <= self.max.x)
                && (position.y >= self.min.y)
                && (position.y <= self.max.y)
                && (position.z >= self.min.z)
                && (position.z <= self.max.z))
    }

    #[inline]
    pub fn is_intersects_aabb(&self, other: &Self) -> bool {
        let self_center = self.center();
        let self_half_extents = self.half_extents();

        let other_half_extents = other.half_extents();
        let other_center = other.center();

        if (self_center.x - other_center.x).abs() > (self_half_extents.x + other_half_extents.x) {
            return false;
        }

        if (self_center.y - other_center.y).abs() > (self_half_extents.y + other_half_extents.y) {
            return false;
        }

        if (self_center.z - other_center.z).abs() > (self_half_extents.z + other_half_extents.z) {
            return false;
        }

        true
    }

    /// Transforms axis-aligned bounding box using given affine transformation matrix.
    ///
    /// # References
    ///
    /// Transforming Axis-Aligned Bounding Boxes by Jim Arvo, "Graphics Gems", Academic Press, 1990
    #[inline]
    #[must_use]
    pub fn transform(&self, m: &Matrix4<f32>) -> AxisAlignedBoundingBox {
        let basis = m.basis();

        let mut transformed = Self {
            min: m.position(),
            max: m.position(),
        };

        for i in 0..3 {
            for j in 0..3 {
                let a = basis[(i, j)] * self.min[j];
                let b = basis[(i, j)] * self.max[j];
                if a < b {
                    transformed.min[i] += a;
                    transformed.max[i] += b;
                } else {
                    transformed.min[i] += b;
                    transformed.max[i] += a;
                }
            }
        }

        transformed
    }

    #[inline]
    pub fn split(&self) -> [AxisAlignedBoundingBox; 8] {
        let center = self.center();
        let min = &self.min;
        let max = &self.max;
        [
            AxisAlignedBoundingBox::from_min_max(
                Vector3::new(min.x, min.y, min.z),
                Vector3::new(center.x, center.y, center.z),
            ),
            AxisAlignedBoundingBox::from_min_max(
                Vector3::new(center.x, min.y, min.z),
                Vector3::new(max.x, center.y, center.z),
            ),
            AxisAlignedBoundingBox::from_min_max(
                Vector3::new(min.x, min.y, center.z),
                Vector3::new(center.x, center.y, max.z),
            ),
            AxisAlignedBoundingBox::from_min_max(
                Vector3::new(center.x, min.y, center.z),
                Vector3::new(max.x, center.y, max.z),
            ),
            AxisAlignedBoundingBox::from_min_max(
                Vector3::new(min.x, center.y, min.z),
                Vector3::new(center.x, max.y, center.z),
            ),
            AxisAlignedBoundingBox::from_min_max(
                Vector3::new(center.x, center.y, min.z),
                Vector3::new(max.x, max.y, center.z),
            ),
            AxisAlignedBoundingBox::from_min_max(
                Vector3::new(min.x, center.y, center.z),
                Vector3::new(center.x, max.y, max.z),
            ),
            AxisAlignedBoundingBox::from_min_max(
                Vector3::new(center.x, center.y, center.z),
                Vector3::new(max.x, max.y, max.z),
            ),
        ]
    }
}

#[cfg(test)]
mod test {
    use crate::aabb::AxisAlignedBoundingBox;
    use nalgebra::{Matrix4, Vector3};

    #[test]
    fn test_aabb_transform() {
        let aabb = AxisAlignedBoundingBox {
            min: Vector3::new(0.0, 0.0, 0.0),
            max: Vector3::new(1.0, 1.0, 1.0),
        };

        let transform = Matrix4::new_translation(&Vector3::new(1.0, 1.0, 1.0))
            * Matrix4::new_nonuniform_scaling(&Vector3::new(2.0, 2.0, 2.0));

        let transformed_aabb = aabb.transform(&transform);

        assert_eq!(transformed_aabb.min, Vector3::new(1.0, 1.0, 1.0));
        assert_eq!(transformed_aabb.max, Vector3::new(3.0, 3.0, 3.0));
    }

    #[test]
    fn test_aabb_default() {
        let _box = AxisAlignedBoundingBox::default();
        assert_eq!(_box.min, Vector3::new(f32::MAX, f32::MAX, f32::MAX));
        assert_eq!(_box.max, Vector3::new(-f32::MAX, -f32::MAX, -f32::MAX));
    }

    #[test]
    fn test_aabb_unit() {
        let _box = AxisAlignedBoundingBox::unit();
        assert_eq!(_box.min, Vector3::new(-0.5, -0.5, -0.5));
        assert_eq!(_box.max, Vector3::new(0.5, 0.5, 0.5));
    }

    #[test]
    fn test_aabb_collapsed() {
        let _box = AxisAlignedBoundingBox::collapsed();
        assert_eq!(_box.min, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(_box.max, Vector3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_aabb_from_radius() {
        let _box = AxisAlignedBoundingBox::from_radius(1.0);
        assert_eq!(_box.min, Vector3::new(-1.0, -1.0, -1.0));
        assert_eq!(_box.max, Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_aabb_from_point() {
        let _box = AxisAlignedBoundingBox::from_point(Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(_box.min, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(_box.max, Vector3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_aabb_from_points() {
        let _box = AxisAlignedBoundingBox::from_points(
            vec![
                Vector3::new(-1.0, -1.0, -1.0),
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(1.0, 1.0, 1.0),
            ]
            .as_ref(),
        );
        assert_eq!(_box.min, Vector3::new(-1.0, -1.0, -1.0));
        assert_eq!(_box.max, Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_aabb_add_point() {
        let mut _box = AxisAlignedBoundingBox::default();
        _box.add_point(Vector3::new(-1.0, -1.0, -1.0));
        _box.add_point(Vector3::new(1.0, 1.0, 1.0));
        assert_eq!(_box.min, Vector3::new(-1.0, -1.0, -1.0));
        assert_eq!(_box.max, Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_aabb_inflate() {
        let mut _box = AxisAlignedBoundingBox::from_radius(1.0);
        _box.inflate(Vector3::new(5.0, 5.0, 5.0));
        assert_eq!(_box.min, Vector3::new(-3.5, -3.5, -3.5));
        assert_eq!(_box.max, Vector3::new(3.5, 3.5, 3.5));
    }

    #[test]
    fn test_aabb_add_box() {
        let mut _box = AxisAlignedBoundingBox::collapsed();
        let _box2 = AxisAlignedBoundingBox::from_radius(1.0);
        _box.add_box(_box2);
        assert_eq!(_box.min, Vector3::new(-1.0, -1.0, -1.0));
        assert_eq!(_box.max, Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_aabb_corners() {
        let _box = AxisAlignedBoundingBox::from_radius(1.0);
        assert_eq!(
            _box.corners(),
            [
                Vector3::new(-1.0, -1.0, -1.0),
                Vector3::new(-1.0, -1.0, 1.0),
                Vector3::new(1.0, -1.0, 1.0),
                Vector3::new(1.0, -1.0, -1.0),
                Vector3::new(-1.0, 1.0, -1.0),
                Vector3::new(-1.0, 1.0, 1.0),
                Vector3::new(1.0, 1.0, 1.0),
                Vector3::new(1.0, 1.0, -1.0),
            ]
        );
        assert_eq!(_box.volume(), 8.0);
        assert_eq!(_box.center(), Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(_box.half_extents(), Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_aabb_offset() {
        let mut _box = AxisAlignedBoundingBox::unit();
        _box.offset(Vector3::new(1.0, 1.0, 1.0));
        assert_eq!(_box.min, Vector3::new(0.5, 0.5, 0.5));
        assert_eq!(_box.max, Vector3::new(1.5, 1.5, 1.5));
    }

    #[test]
    fn test_aabb_invalidate() {
        let mut _box = AxisAlignedBoundingBox::collapsed();
        _box.invalidate();
        assert_eq!(_box.min, Vector3::new(f32::MAX, f32::MAX, f32::MAX));
        assert_eq!(_box.max, Vector3::new(-f32::MAX, -f32::MAX, -f32::MAX));
    }

    #[test]
    fn test_aabb_is_valid() {
        let mut _box = AxisAlignedBoundingBox::default();
        assert!(!_box.is_valid());

        _box.add_point(Vector3::new(1.0, 1.0, 1.0));
        assert!(_box.is_valid());

        _box.add_point(Vector3::new(-1.0, -1.0, -1.0));
        assert!(_box.is_valid());
    }

    #[test]
    fn test_aabb_is_degenerate() {
        let _box = AxisAlignedBoundingBox::unit();
        assert!(!_box.is_degenerate());

        let _box = AxisAlignedBoundingBox::collapsed();
        assert!(_box.is_degenerate());
    }

    #[test]
    fn test_aabb_is_invalid_or_degenerate() {
        let mut _box = AxisAlignedBoundingBox::collapsed();
        assert!(_box.is_invalid_or_degenerate());

        _box.invalidate();
        assert!(_box.is_invalid_or_degenerate());
    }

    #[test]
    fn test_aabb_is_contains_point() {
        let _box = AxisAlignedBoundingBox::unit();
        assert!(_box.is_contains_point(Vector3::new(0.0, 0.0, 0.0)));

        for point in _box.corners() {
            assert!(_box.is_contains_point(point));
        }
    }

    #[test]
    fn test_aabb_is_intersects_sphere() {
        let _box = AxisAlignedBoundingBox::unit();
        assert!(_box.is_intersects_sphere(Vector3::new(0.0, 0.0, 0.0), 1.0));
        assert!(_box.is_intersects_sphere(Vector3::new(0.0, 0.0, 0.0), 0.5));
        assert!(_box.is_intersects_sphere(Vector3::new(0.0, 0.0, 0.0), 1.5));
        assert!(_box.is_intersects_sphere(Vector3::new(0.5, 0.5, 0.5), 1.0));
        assert!(_box.is_intersects_sphere(Vector3::new(0.25, 0.25, 0.25), 1.0));

        assert!(!_box.is_intersects_sphere(Vector3::new(10.0, 10.0, 10.0), 1.0));
        assert!(!_box.is_intersects_sphere(Vector3::new(-10.0, -10.0, -10.0), 1.0));
    }

    #[test]
    fn test_aabb_is_intersects_aabb() {
        let _box = AxisAlignedBoundingBox::unit();
        let mut _box2 = _box;
        assert!(_box.is_intersects_aabb(&_box2));

        _box2.offset(Vector3::new(0.5, 0.0, 0.0));
        assert!(_box.is_intersects_aabb(&_box2));
        _box2.offset(Vector3::new(1.0, 0.0, 0.0));
        assert!(!_box.is_intersects_aabb(&_box2));

        let mut _box2 = _box;
        _box2.offset(Vector3::new(0.0, 0.5, 0.0));
        assert!(_box.is_intersects_aabb(&_box2));
        _box2.offset(Vector3::new(0.0, 1.0, 0.0));
        assert!(!_box.is_intersects_aabb(&_box2));

        let mut _box2 = _box;
        _box2.offset(Vector3::new(0.0, 0.0, 0.5));
        assert!(_box.is_intersects_aabb(&_box2));
        _box2.offset(Vector3::new(0.0, 0.0, 1.0));
        assert!(!_box.is_intersects_aabb(&_box2));
    }

    #[test]
    fn test_aabb_split() {
        let _box = AxisAlignedBoundingBox::from_radius(1.0);
        let _boxes = _box.split();

        assert_eq!(_boxes[0].min, Vector3::new(-1.0, -1.0, -1.0));
        assert_eq!(_boxes[0].max, Vector3::new(0.0, 0.0, 0.0));

        assert_eq!(_boxes[1].min, Vector3::new(0.0, -1.0, -1.0));
        assert_eq!(_boxes[1].max, Vector3::new(1.0, 0.0, 0.0));

        assert_eq!(_boxes[2].min, Vector3::new(-1.0, -1.0, 0.0));
        assert_eq!(_boxes[2].max, Vector3::new(0.0, 0.0, 1.0));

        assert_eq!(_boxes[3].min, Vector3::new(0.0, -1.0, 0.0));
        assert_eq!(_boxes[3].max, Vector3::new(1.0, 0.0, 1.0));

        assert_eq!(_boxes[4].min, Vector3::new(-1.0, 0.0, -1.0));
        assert_eq!(_boxes[4].max, Vector3::new(0.0, 1.0, 0.0));

        assert_eq!(_boxes[5].min, Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(_boxes[5].max, Vector3::new(1.0, 1.0, 0.0));

        assert_eq!(_boxes[6].min, Vector3::new(-1.0, 0.0, 0.0));
        assert_eq!(_boxes[6].max, Vector3::new(0.0, 1.0, 1.0));

        assert_eq!(_boxes[7].min, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(_boxes[7].max, Vector3::new(1.0, 1.0, 1.0));
    }
}
