use crate::camera;
use fyrox::core::{algebra::Vector3, inspect::prelude::*, reflect::prelude::*};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct SceneCameraSettings {
    pub position: Vector3<f32>,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for SceneCameraSettings {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 1.0, camera::DEFAULT_Z_OFFSET),
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Inspect, Reflect)]
pub struct CameraSettings {
    pub speed: f32,
    pub invert_dragging: bool,
    pub drag_speed: f32,
    #[inspect(skip)]
    #[reflect(hidden)]
    pub camera_settings: HashMap<PathBuf, SceneCameraSettings>,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            speed: 10.0,
            invert_dragging: false,
            drag_speed: 0.01,
            camera_settings: Default::default(),
        }
    }
}
