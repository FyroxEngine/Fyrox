use fyrox::core::reflect::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct GeneralSettings {
    #[reflect(
        description = "Defines whether the editor checks for references to an object that is about to be deleted or not."
    )]
    pub show_node_removal_dialog: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            show_node_removal_dialog: true,
        }
    }
}
