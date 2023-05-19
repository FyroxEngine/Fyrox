use fyrox::core::{algebra::Vector2, reflect::prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Reflect)]
pub struct WindowsSettings {
    #[serde(default)]
    pub window_position: Vector2<f32>,
    #[serde(default)]
    pub window_size: Vector2<f32>,
}

impl Default for WindowsSettings {
    fn default() -> Self {
        Self {
            window_position: Vector2::new(0.0, 0.0),
            window_size: Vector2::new(1024.0, 768.0),
        }
    }
}
