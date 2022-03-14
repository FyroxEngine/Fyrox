use fyrox::{
    core::inspect::{Inspect, PropertyInfo},
    gui::inspector::{FieldKind, PropertyChanged},
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Inspect)]
pub struct RotateInteractionModeSettings {
    pub angle_snapping: bool,
    pub x_snap_step: f32,
    pub y_snap_step: f32,
    pub z_snap_step: f32,
}

impl Default for RotateInteractionModeSettings {
    fn default() -> Self {
        Self {
            angle_snapping: false,
            x_snap_step: 2.5,
            y_snap_step: 2.5,
            z_snap_step: 2.5,
        }
    }
}

impl RotateInteractionModeSettings {
    pub fn handle_property_changed(&mut self, property_changed: &PropertyChanged) -> bool {
        if let FieldKind::Object(ref args) = property_changed.value {
            return match property_changed.name.as_ref() {
                Self::ANGLE_SNAPPING => args.try_override(&mut self.angle_snapping),
                Self::X_SNAP_STEP => args.try_override(&mut self.x_snap_step),
                Self::Y_SNAP_STEP => args.try_override(&mut self.y_snap_step),
                Self::Z_SNAP_STEP => args.try_override(&mut self.z_snap_step),
                _ => false,
            };
        }
        false
    }
}
