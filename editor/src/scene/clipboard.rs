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

use crate::fyrox::{
    core::pool::Handle,
    scene::{graph::Graph, node::Node, Scene},
};
use crate::{scene::GraphSelection, Engine};
use std::collections::HashMap;

pub struct Clipboard {
    graph: Graph,
    empty: bool,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self {
            graph: Graph::new(),
            empty: true,
        }
    }
}

#[derive(Default, Debug)]
pub struct DeepCloneResult {
    pub root_nodes: Vec<Handle<Node>>,
}

fn deep_clone_nodes(
    root_nodes: &[Handle<Node>],
    source_graph: &Graph,
    dest_graph: &mut Graph,
) -> DeepCloneResult {
    let mut result = DeepCloneResult::default();

    let mut old_new_mapping = HashMap::new();

    for &root_node in root_nodes.iter() {
        let (_, old_to_new) = source_graph.copy_node(
            root_node,
            dest_graph,
            &mut |_, _| true,
            &mut |_, _| {},
            &mut |_, _, _| {},
        );
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
    pub fn fill_from_selection(
        &mut self,
        selection: &GraphSelection,
        scene_handle: Handle<Scene>,
        engine: &Engine,
    ) {
        self.clear();

        let scene = &engine.scenes[scene_handle];

        let root_nodes = selection.root_nodes(&scene.graph);

        deep_clone_nodes(&root_nodes, &scene.graph, &mut self.graph);

        self.empty = false;
    }

    pub fn paste(&mut self, dest_graph: &mut Graph) -> DeepCloneResult {
        assert!(!self.empty);

        deep_clone_nodes(
            self.graph[self.graph.get_root()].children(),
            &self.graph,
            dest_graph,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }

    pub fn clear(&mut self) {
        self.empty = true;
        self.graph = Graph::new();
    }
}
