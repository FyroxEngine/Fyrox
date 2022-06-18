use fyrox::{
    core::inspect::{Inspect, PropertyInfo},
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
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
        handle_object_property_changed!(self, property_changed,
            Self::ANGLE_SNAPPING => angle_snapping,
            Self::X_SNAP_STEP => x_snap_step,
            Self::Y_SNAP_STEP => y_snap_step,
            Self::Z_SNAP_STEP => z_snap_step
        )
    }
}
