use std::ops;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    #[inline]
    pub fn new() -> Self {
        Vec2 {
            x: 0.0,
            y: 0.0,
        }
    }

    #[inline]
    pub fn make(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }

    #[inline]
    pub fn dot(&self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn len(&self) -> f32 {
        self.dot(*self).sqrt()
    }

    #[inline]
    pub fn angle(&self, other: Self) -> f32 {
        (self.dot(other) / (self.len() * other.len())).acos()
    }

    #[inline]
    pub fn perpendicular(&self) -> Vec2 {
        Vec2 { x: self.y, y: -self.x }
    }

    #[inline]
    pub fn scale(&self, scalar: f32) -> Vec2 {
        Vec2 { x: self.x * scalar, y: self.y * scalar }
    }

    #[inline]
    pub fn normalized(&self) -> Option<Vec2> {
        let len = self.len();
        if len >= std::f32::EPSILON {
            let inv_len = 1.0 / len;
            return Some(Vec2 {
                x: self.x * inv_len,
                y: self.y * inv_len,
            });
        }
        None
    }
}

impl ops::Add<Self> for Vec2 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self {
            x: self.x + b.x,
            y: self.y + b.y,
        }
    }
}

impl ops::AddAssign<Self> for Vec2 {
    fn add_assign(&mut self, b: Self) {
        self.x += b.x;
        self.y += b.y;
    }
}

impl ops::Sub<Self> for Vec2 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self {
            x: self.x - b.x,
            y: self.y - b.y,
        }
    }
}

impl ops::SubAssign<Self> for Vec2 {
    fn sub_assign(&mut self, b: Self) {
        self.x -= b.x;
        self.y -= b.y;
    }
}
