use crate::fyrox::core::reflect::prelude::*;
use serde::{Deserialize, Serialize};
use std::ops::Range;

fn default_zoom_speed() -> f32 {
    0.5
}

fn default_zoom_range() -> Range<f32> {
    0.0f32..100.0f32
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
    #[reflect(min_value = 0.0, max_value = 1000.0)]
    #[serde(default = "default_zoom_range")]
    pub zoom_range: Range<f32>,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            speed: 10.0,
            invert_dragging: false,
            drag_speed: 0.01,
            zoom_speed: default_zoom_speed(),
            zoom_range: default_zoom_range(),
        }
    }
}
