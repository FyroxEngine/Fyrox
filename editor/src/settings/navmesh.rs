use fyrox::core::{
    inspect::{Inspect, PropertyInfo},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Inspect, Reflect)]
pub struct NavmeshSettings {
    #[inspect(
        description = "Show all navigational meshes in scene. With this function turned off, only currently edited navmesh will be shown."
    )]
    pub draw_all: bool,

    #[inspect(description = "Radius of a nav mesh vertex.")]
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
