use crate::{
    algebra::{Matrix4, Vector3},
    math::Matrix4Ext,
    visitor::{Visit, VisitResult, Visitor},
};

#[derive(Copy, Clone, Debug, Visit)]
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

        self.max.x > self.min.x
            && self.max.y > self.min.y
            && self.max.z > self.min.z
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
    pub fn intersect_aabb(&self, other: &Self) -> bool {
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
    use crate::algebra::{Matrix4, Vector3};
    use crate::math::aabb::AxisAlignedBoundingBox;

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
}
