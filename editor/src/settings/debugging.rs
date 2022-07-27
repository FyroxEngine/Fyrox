use fyrox::{
    core::{
        inspect::{Inspect, PropertyInfo},
        reflect::Reflect,
    },
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Inspect, Reflect)]
pub struct DebuggingSettings {
    pub show_physics: bool,
    pub show_bounds: bool,
    pub show_tbn: bool,
}

impl Default for DebuggingSettings {
    fn default() -> Self {
        Self {
            show_physics: true,
            show_bounds: true,
            show_tbn: false,
        }
    }
}

impl DebuggingSettings {
    pub fn handle_property_changed(&mut self, property_changed: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, property_changed,
            Self::SHOW_PHYSICS => show_physics,
            Self::SHOW_BOUNDS => show_bounds,
            Self::SHOW_TBN => show_tbn
        )
    }
}
