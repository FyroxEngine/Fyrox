use crate::fyrox::{core::reflect::prelude::*, renderer::QualitySettings};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Reflect)]
pub struct GraphicsSettings {
    pub quality: QualitySettings,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            quality: Default::default(),
            z_near: 0.025,
            z_far: 128.0,
        }
    }
}
