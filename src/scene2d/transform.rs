use crate::core::{algebra::Vector2, visitor::prelude::*};

#[derive(Visit)]
pub struct Transform {
    position: Vector2<f32>,
    scale: Vector2<f32>,
    rotation: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vector2::new(0.0, 0.0),
            scale: Vector2::new(1.0, 1.0),
            rotation: 0.0,
        }
    }
}

impl Transform {
    pub fn set_position(&mut self, position: Vector2<f32>) {
        self.position = position;
    }
}
