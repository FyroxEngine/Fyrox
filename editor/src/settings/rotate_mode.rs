use crate::fyrox::core::reflect::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Reflect)]
pub struct RotateInteractionModeSettings {
    pub angle_snapping: bool,
    pub x_snap_step: f32,
    pub y_snap_step: f32,
    pub z_snap_step: f32,
}

impl Default for RotateInteractionModeSettings {
    fn default() -> Self {
        Self {
            angle_snapping: false,
            x_snap_step: 2.5,
            y_snap_step: 2.5,
            z_snap_step: 2.5,
        }
    }
}
