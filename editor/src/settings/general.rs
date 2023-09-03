use fyrox::core::reflect::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct GeneralSettings {
    #[reflect(
        description = "Defines whether the editor checks for references to an object that is about to be deleted or not."
    )]
    pub show_node_removal_dialog: bool,

    #[reflect(
        description = "When set, suspends the editor execution if its main window is unfocused. Use this option to reduce \
    CPU/GPU resources consumption when you don't need the editor to run in the background."
    )]
    #[serde(default = "default_suspension_state")]
    pub suspend_unfocused_editor: bool,
}

fn default_suspension_state() -> bool {
    true
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            show_node_removal_dialog: true,
            suspend_unfocused_editor: default_suspension_state(),
        }
    }
}
