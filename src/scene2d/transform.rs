use crate::core::algebra::{Matrix3, Matrix4, Vector3};
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
    pub fn set_position(&mut self, position: Vector2<f32>) -> &mut Self {
        self.position = position;
        self
    }

    pub fn set_rotation(&mut self, rotation: f32) -> &mut Self {
        self.rotation = rotation;
        self
    }

    pub fn set_scale(&mut self, scale: Vector2<f32>) -> &mut Self {
        self.scale = scale;
        self
    }

    pub fn offset(&mut self, offset: Vector2<f32>) -> &mut Self {
        self.position += offset;
        self
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        Matrix4::new_nonuniform_scaling(&Vector3::new(self.scale.x, self.scale.y, 1.0))
            * Matrix3::new_rotation(self.rotation).to_homogeneous()
            * Matrix4::new_translation(&Vector3::new(self.position.x, self.position.y, 0.0))
    }
}

pub struct TransformBuilder {
    position: Vector2<f32>,
    scale: Vector2<f32>,
    rotation: f32,
}

impl TransformBuilder {
    pub fn new() -> Self {
        Self {
            position: Default::default(),
            scale: Vector2::new(1.0, 1.0),
            rotation: 0.0,
        }
    }

    pub fn with_position(mut self, position: Vector2<f32>) -> Self {
        self.position = position;
        self
    }

    pub fn with_scale(mut self, scale: Vector2<f32>) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn build(self) -> Transform {
        Transform {
            position: self.position,
            scale: self.scale,
            rotation: self.rotation,
        }
    }
}
