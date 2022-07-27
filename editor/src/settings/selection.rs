use fyrox::{
    core::{inspect::prelude::*, reflect::Reflect},
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Inspect, Reflect)]
pub struct SelectionSettings {
    pub ignore_back_faces: bool,
}

impl SelectionSettings {
    pub fn handle_property_changed(&mut self, property_changed: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, property_changed, Self::IGNORE_BACK_FACES => ignore_back_faces)
    }
}
