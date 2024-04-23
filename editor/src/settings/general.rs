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

    #[serde(default = "default_max_history_entries")]
    #[reflect(
        description = "Maximum capacity of actions history stack stored by the editor. Default is \
    512. Large numbers will increase maximum amount of memory needed for the editor."
    )]
    pub max_history_entries: usize,

    #[reflect(
        description = "Enables or disables preview generation for assets in the asset browser. \
    This feature could be disabled, if you're getting performance issues when there's a lot of assets \
    in a folder. Due to technical difficulties, preview generation will be done on main thread, which \
    could cause lag spikes."
    )]
    #[serde(default = "default_generate_previews")]
    pub generate_previews: bool,
}

fn default_suspension_state() -> bool {
    true
}

fn default_script_editor() -> ScriptEditor {
    ScriptEditor::SystemDefault
}

fn default_max_history_entries() -> usize {
    512
}

fn default_generate_previews() -> bool {
    true
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
            max_history_entries: default_max_history_entries(),
            generate_previews: default_generate_previews(),
        }
    }
}
