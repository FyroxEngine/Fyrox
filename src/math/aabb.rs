use crate::{
    math::vec3::Vec3,
    visitor::{Visit, Visitor, VisitResult}
};

#[derive(Copy, Clone)]
pub struct AxisAlignedBoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for AxisAlignedBoundingBox {
    fn default() -> Self {
        Self {
            min: Vec3::new(std::f32::MAX, std::f32::MAX, std::f32::MAX),
            max: Vec3::new(-std::f32::MAX, -std::f32::MAX, -std::f32::MAX)
        }
    }
}

impl AxisAlignedBoundingBox {
    pub fn from_min_max(min: Vec3, max: Vec3) -> Self {
        Self {
            min,
            max,
        }
    }

    pub fn from_points(points: &[Vec3]) -> Self {
        let mut aabb = AxisAlignedBoundingBox::default();
        for pt in points {
            aabb.add_point(*pt);
        }
        aabb
    }

    pub fn add_point(&mut self, a: Vec3) {
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

    pub fn corners(&self) -> [Vec3; 8] {
        [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
        ]
    }

    pub fn offset(&mut self, v: Vec3) {
        self.min += v;
        self.max += v;
    }

    pub fn center(&self) -> Vec3 {
        (self.max + self.min).scale(0.5)
    }

    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min).scale(0.5)
    }

    pub fn invalidate(&mut self) {
        *self = Default::default();
    }

    pub fn is_contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
            point.y >= self.min.y && point.y <= self.max.y &&
            point.z >= self.min.z && point.z <= self.max.z
    }

    pub fn is_intersects_sphere(&self, position: Vec3, radius: f32) -> bool {
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

        dmin <= r2 ||
            ((position.x >= self.min.x) && (position.x <= self.max.x) && (position.y >= self.min.y) &&
                (position.y <= self.max.y) && (position.z >= self.min.z) && (position.z <= self.max.z))
    }

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
}

impl Visit for AxisAlignedBoundingBox {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.min.visit("Min", visitor)?;
        self.max.visit("Max", visitor)?;

        visitor.leave_region()
    }
}