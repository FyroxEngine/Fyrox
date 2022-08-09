use fyrox::core::{inspect::prelude::*, reflect::Reflect};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Inspect, Reflect)]
pub struct CameraSettings {
    pub invert_dragging: bool,
    pub drag_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            invert_dragging: false,
            drag_speed: 0.01,
        }
    }
}
