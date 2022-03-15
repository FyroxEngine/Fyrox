use fyrox::{
    core::inspect::{Inspect, PropertyInfo},
    gui::inspector::{FieldKind, PropertyChanged},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Inspect)]
pub struct SelectionSettings {
    pub ignore_back_faces: bool,
}

impl SelectionSettings {
    pub fn handle_property_changed(&mut self, property_changed: &PropertyChanged) -> bool {
        if let FieldKind::Object(ref args) = property_changed.value {
            return match property_changed.name.as_ref() {
                Self::IGNORE_BACK_FACES => args.try_override(&mut self.ignore_back_faces),
                _ => false,
            };
        }
        false
    }
}
