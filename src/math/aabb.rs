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

    pub fn invalidate(&mut self) {
        *self = Default::default();
    }

    pub fn is_contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x && point.y >= self.min.y &&
            point.y <= self.max.y && point.z >= self.min.z && point.z <= self.max.z
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
}

impl Visit for AxisAlignedBoundingBox {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.min.visit("Min", visitor)?;
        self.max.visit("Max", visitor)?;

        visitor.leave_region()
    }
}