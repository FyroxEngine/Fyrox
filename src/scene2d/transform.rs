use crate::core::algebra::Vector2;
use crate::core::visitor::{Visit, VisitResult, Visitor};

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

impl Visit for Transform {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;
        self.scale.visit("Scale", visitor)?;
        self.rotation.visit("Rotation", visitor)?;

        visitor.leave_region()
    }
}

impl Transform {
    pub fn set_position(&mut self, position: Vector2<f32>) {
        self.position = position;
    }
}
