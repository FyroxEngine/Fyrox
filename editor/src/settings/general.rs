// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::fyrox::core::{reflect::prelude::*, type_traits::prelude::*, uuid_provider};
use serde::{Deserialize, Serialize};
use strum_macros::{AsRefStr, EnumString, VariantNames};

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
    TypeUuidProvider,
)]
#[type_uuid(id = "35e8d30d-1213-4d87-905e-19d48550e6d5")]
pub enum EditorStyle {
    Dark,
    Light,
}

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

    #[reflect(
        description = "Maximum amount of log entries. Large values could harm performance and \
        increase memory usage. Typical values are 200-500."
    )]
    #[serde(default = "default_max_log_entries")]
    pub max_log_entries: usize,

    #[serde(default = "default_style")]
    pub style: EditorStyle,
}

fn default_style() -> EditorStyle {
    EditorStyle::Dark
}

fn default_max_log_entries() -> usize {
    256
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
    Zed,
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
            max_log_entries: default_max_log_entries(),
            style: EditorStyle::Dark,
        }
    }
}
