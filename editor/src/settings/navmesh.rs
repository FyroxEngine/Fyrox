use crate::fyrox::core::reflect::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct NavmeshSettings {
    #[reflect(
        description = "Show all navigational meshes in scene. With this function turned off, only currently edited navmesh will be shown."
    )]
    pub draw_all: bool,

    #[reflect(description = "Radius of a nav mesh vertex.")]
    pub vertex_radius: f32,
}

impl Default for NavmeshSettings {
    fn default() -> Self {
        Self {
            draw_all: true,
            vertex_radius: 0.2,
        }
    }
}
