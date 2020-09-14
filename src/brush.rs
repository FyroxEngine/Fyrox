use crate::core::{color::Color, math::vec2::Vec2};

#[derive(Clone, Debug, PartialEq)]
pub struct GradientPoint {
    pub stop: f32,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Brush {
    Solid(Color),
    LinearGradient {
        from: Vec2,
        to: Vec2,
        stops: Vec<GradientPoint>,
    },
    RadialGradient {
        center: Vec2,
        stops: Vec<GradientPoint>,
    },
}
