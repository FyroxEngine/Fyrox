use crate::fyrox::core::reflect::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct DebuggingSettings {
    pub show_physics: bool,
    pub show_bounds: bool,
    pub show_tbn: bool,
    #[serde(default)]
    pub show_terrains: bool,
    #[serde(default)]
    pub show_light_bounds: bool,
    #[serde(default)]
    pub show_camera_bounds: bool,
    #[reflect(description = "Size of pictograms in meters. It is used for objects like lights.")]
    #[serde(default)]
    pub pictogram_size: f32,
    #[reflect(
        description = "Forces the editor to save the scene in text form as well as standard binary."
    )]
    #[serde(default)]
    pub save_scene_in_text_form: bool,
}

impl Default for DebuggingSettings {
    fn default() -> Self {
        Self {
            show_physics: true,
            show_bounds: true,
            show_tbn: false,
            show_terrains: false,
            show_light_bounds: true,
            show_camera_bounds: true,
            pictogram_size: 0.33,
            save_scene_in_text_form: false,
        }
    }
}
