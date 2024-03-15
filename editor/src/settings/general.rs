use crate::fyrox::core::{reflect::prelude::*, uuid_provider};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

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

    #[serde(default = "default_script_editor")]
    pub script_editor: ScriptEditor,
}

fn default_suspension_state() -> bool {
    true
}

fn default_script_editor() -> ScriptEditor {
    ScriptEditor::SystemDefault
}

#[derive(
    Copy,
    Clone,
    Hash,
    PartialOrd,
    PartialEq,
    Eq,
    Ord,
    Debug,
    Serialize,
    Deserialize,
    Reflect,
    AsRefStr,
    EnumString,
    VariantNames,
)]
pub enum ScriptEditor {
    SystemDefault,
    VSCode,
    Emacs,
    XCode,
}

uuid_provider!(ScriptEditor = "d0c942e8-24e4-40f2-ad2e-1b9f189d3ca2");

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            show_node_removal_dialog: true,
            suspend_unfocused_editor: default_suspension_state(),
            script_editor: default_script_editor(),
        }
    }
}
