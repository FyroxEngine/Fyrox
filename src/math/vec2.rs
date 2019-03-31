#[derive(Copy, Clone)]
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
}