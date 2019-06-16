use std::ops;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new() -> Self {
        Vec2 {
            x: 0.0,
            y:0.0
        }
    }

    pub fn make(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }

    pub fn dot(&self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn len(&self) -> f32 {
        self.dot(*self).sqrt()
    }

    pub fn angle(&self, other: Self) -> f32 {
        (self.dot(other) / (self.len() * other.len())).acos()
    }
}

impl ops::Add<Self> for Vec2 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self {
            x: self.x + b.x,
            y: self.y + b.y
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
            y: self.y - b.y
        }
    }
}

impl ops::SubAssign<Self> for Vec2 {
    fn sub_assign(&mut self, b: Self) {
        self.x -= b.x;
        self.y -= b.y;
    }
}
