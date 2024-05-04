use crate::fyrox::{core::reflect::prelude::*, renderer::QualitySettings};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Reflect)]
pub struct GraphicsSettings {
    pub quality: QualitySettings,
    pub z_near: f32,
    pub z_far: f32,
    #[serde(default = "default_draw_grid")]
    pub draw_grid: bool,
}

fn default_draw_grid() -> bool {
    true
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            quality: Default::default(),
            z_near: 0.025,
            z_far: 128.0,
            draw_grid: default_draw_grid(),
        }
    }
}
