use fyrox::core::{inspect::prelude::*, reflect::prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Inspect, Reflect, Eq)]
pub struct SelectionSettings {
    pub ignore_back_faces: bool,

    // Hidden because there's a separate switch in world viewer for this.
    #[inspect(skip)]
    #[reflect(hidden)]
    pub track_selection: bool,
}

impl Default for SelectionSettings {
    fn default() -> Self {
        Self {
            ignore_back_faces: false,
            track_selection: true,
        }
    }
}
