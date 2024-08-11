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

use crate::fyrox::core::pool::Handle;
use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::gui::{UiNode, UserInterface};
use crate::ui_scene::selection::UiSelection;
use std::collections::HashMap;

pub struct Clipboard {
    ui: UserInterface,
    empty: bool,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            ui: UserInterface::new(Default::default()),
            empty: true,
        }
    }
}

#[derive(Default, Debug)]
pub struct DeepCloneResult {
    pub root_nodes: Vec<Handle<UiNode>>,
}

fn deep_clone_nodes(
    root_nodes: &[Handle<UiNode>],
    source_graph: &UserInterface,
    dest_ui: &mut UserInterface,
) -> DeepCloneResult {
    let mut result = DeepCloneResult::default();

    let mut old_new_mapping = HashMap::new();

    for &root_node in root_nodes.iter() {
        let (_, old_to_new) = source_graph.copy_node_to(root_node, dest_ui, &mut |_, _, _| {});
        // Merge mappings.
        for (old, new) in old_to_new.into_inner() {
            old_new_mapping.insert(old, new);
        }
    }

    result.root_nodes = root_nodes
        .iter()
        .map(|n| *old_new_mapping.get(n).unwrap())
        .collect::<Vec<_>>();

    result
}

impl Clipboard {
    pub fn fill_from_selection(&mut self, selection: &UiSelection, ui: &UserInterface) {
        self.clear();

        let root_widgets = selection.root_widgets(ui);

        deep_clone_nodes(&root_widgets, ui, &mut self.ui);

        self.empty = false;
    }

    pub fn paste(&mut self, dest_ui: &mut UserInterface) -> DeepCloneResult {
        assert!(!self.empty);

        deep_clone_nodes(self.ui.node(self.ui.root()).children(), &self.ui, dest_ui)
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    pub fn clear(&mut self) {
        self.empty = true;
        self.ui = UserInterface::new(Default::default());
    }
}
