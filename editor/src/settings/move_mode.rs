use fyrox::{
    core::inspect::{Inspect, PropertyInfo},
    gui::inspector::PropertyChanged,
    handle_object_property_changed,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Inspect)]
pub struct MoveInteractionModeSettings {
    pub grid_snapping: bool,
    pub x_snap_step: f32,
    pub y_snap_step: f32,
    pub z_snap_step: f32,
}

impl Default for MoveInteractionModeSettings {
    fn default() -> Self {
        Self {
            grid_snapping: false,
            x_snap_step: 0.05,
            y_snap_step: 0.05,
            z_snap_step: 0.05,
        }
    }
}

impl MoveInteractionModeSettings {
    pub fn handle_property_changed(&mut self, property_changed: &PropertyChanged) -> bool {
        handle_object_property_changed!(self, property_changed,
            Self::GRID_SNAPPING => grid_snapping,
            Self::X_SNAP_STEP => x_snap_step,
            Self::Y_SNAP_STEP => y_snap_step,
            Self::Z_SNAP_STEP => z_snap_step
        )
    }
}
