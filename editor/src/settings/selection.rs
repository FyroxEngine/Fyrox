use crate::fyrox::core::reflect::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect, Eq)]
pub struct SelectionSettings {
    pub ignore_back_faces: bool,

    // Hidden because there's a separate switch in world viewer for this.
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
