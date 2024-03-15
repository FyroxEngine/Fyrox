use crate::fyrox::core::{algebra::Vector3, math, reflect::prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Reflect)]
pub struct MoveInteractionModeSettings {
    pub grid_snapping: bool,
    pub x_snap_step: f32,
    pub y_snap_step: f32,
    pub z_snap_step: f32,
}

impl Default for MoveInteractionModeSettings {
    fn default() -> Self {
        Self {
            grid_snapping: false,
            x_snap_step: 0.05,
            y_snap_step: 0.05,
            z_snap_step: 0.05,
        }
    }
}

impl MoveInteractionModeSettings {
    pub fn try_snap_vector_to_grid(&self, v: Vector3<f32>) -> Vector3<f32> {
        if self.grid_snapping {
            Vector3::new(
                math::round_to_step(v.x, self.x_snap_step),
                math::round_to_step(v.y, self.y_snap_step),
                math::round_to_step(v.z, self.z_snap_step),
            )
        } else {
            v
        }
    }
}
