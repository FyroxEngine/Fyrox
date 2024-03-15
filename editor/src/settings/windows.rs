use crate::fyrox::{
    core::{algebra::Vector2, reflect::prelude::*},
    gui::dock::config::DockingManagerLayoutDescriptor,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Reflect)]
pub struct WindowsSettings {
    #[serde(default)]
    pub window_position: Vector2<f32>,
    #[serde(default)]
    pub window_size: Vector2<f32>,
    #[serde(default)]
    #[reflect(hidden)]
    pub layout: Option<DockingManagerLayoutDescriptor>,
}

impl Default for WindowsSettings {
    fn default() -> Self {
        Self {
            window_position: Vector2::new(0.0, 0.0),
            window_size: Vector2::new(1024.0, 768.0),
            layout: None,
        }
    }
}
