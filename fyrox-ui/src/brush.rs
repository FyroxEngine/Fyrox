use crate::core::algebra::Vector2;
use crate::core::color::Color;

#[derive(Clone, Debug, PartialEq)]
pub struct GradientPoint {
    pub stop: f32,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Brush {
    Solid(Color),
    LinearGradient {
        from: Vector2<f32>,
        to: Vector2<f32>,
        stops: Vec<GradientPoint>,
    },
    RadialGradient {
        center: Vector2<f32>,
        stops: Vec<GradientPoint>,
    },
}
