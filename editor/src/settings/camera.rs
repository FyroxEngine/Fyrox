use fyrox::core::reflect::prelude::*;
use serde::{Deserialize, Serialize};

fn default_zoom_speed() -> f32 {
    0.5
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct CameraSettings {
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub invert_dragging: bool,
    #[serde(default)]
    pub drag_speed: f32,
    #[serde(default = "default_zoom_speed")]
    pub zoom_speed: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            speed: 10.0,
            invert_dragging: false,
            drag_speed: 0.01,
            zoom_speed: default_zoom_speed(),
        }
    }
}
