use fyrox::core::{inspect::prelude::*, reflect::Reflect};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Inspect, Reflect)]
pub struct CameraSettings {
    pub speed: f32,
    pub invert_dragging: bool,
    pub drag_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            speed: 10.0,
            invert_dragging: false,
            drag_speed: 0.01,
        }
    }
}
