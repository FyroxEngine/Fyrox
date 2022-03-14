use fyrox::{
    core::inspect::{Inspect, PropertyInfo},
    gui::inspector::{FieldKind, PropertyChanged},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Inspect)]
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
        if let FieldKind::Object(ref args) = property_changed.value {
            return match property_changed.name.as_ref() {
                Self::SHOW_PHYSICS => args.try_override(&mut self.show_physics),
                Self::SHOW_BOUNDS => args.try_override(&mut self.show_bounds),
                Self::SHOW_TBN => args.try_override(&mut self.show_tbn),
                _ => false,
            };
        }
        false
    }
}
