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

use crate::{
    command::{Command, SetPropertyCommand},
    fyrox::{
        asset::core::algebra::Vector3,
        core::{
            algebra::UnitQuaternion, math::Matrix4Ext, pool::Handle, reflect::Reflect,
            some_or_return,
        },
        engine::Engine,
        graph::BaseSceneGraph,
        gui::inspector::PropertyChanged,
        scene::{graph::Graph, node::Node, SceneContainer},
    },
    message::MessageSender,
    scene::{
        commands::GameSceneContext, controller::SceneController, GameScene, SelectionContainer,
    },
    utils,
};

#[derive(Debug, Default, Clone, Eq)]
pub struct GraphSelection {
    pub nodes: Vec<Handle<Node>>,
}

impl SelectionContainer for GraphSelection {
    fn len(&self) -> usize {
        self.nodes.len()
    }

    fn first_selected_entity(
        &self,
        controller: &dyn SceneController,
        scenes: &SceneContainer,
        callback: &mut dyn FnMut(&dyn Reflect, bool),
    ) {
        let game_scene = some_or_return!(controller.downcast_ref::<GameScene>());
        let scene = &scenes[game_scene.scene];
        if let Some(node) = self
            .nodes
            .first()
            .and_then(|handle| scene.graph.try_get_node(*handle))
        {
            (callback)(node as &dyn Reflect, node.has_inheritance_parent());
        }
    }

    fn on_property_changed(
        &mut self,
        controller: &mut dyn SceneController,
        args: &PropertyChanged,
        engine: &mut Engine,
        sender: &MessageSender,
    ) {
        let game_scene = some_or_return!(controller.downcast_mut::<GameScene>());
        let scene = &mut engine.scenes[game_scene.scene];

        let group = self
            .nodes
            .iter()
            .filter_map(|&node_handle| {
                game_scene.node_property_changed_handler.handle(
                    args,
                    node_handle,
                    scene.graph.try_get_mut(node_handle)?,
                )
            })
            .collect::<Vec<_>>();

        sender.do_command_group_with_inheritance(group, args);
    }

    fn paste_property(&mut self, path: &str, value: &dyn Reflect, sender: &MessageSender) {
        let group = self
            .nodes
            .iter()
            .filter_map(|&node_handle| {
                value.try_clone_box().map(|value| {
                    Command::new(SetPropertyCommand::new(
                        path.to_string(),
                        value,
                        move |ctx| {
                            ctx.get_mut::<GameSceneContext>()
                                .scene
                                .graph
                                .try_get_mut(node_handle)
                                .map(|n| n as &mut dyn Reflect)
                        },
                    ))
                })
            })
            .collect::<Vec<_>>();

        sender.do_command_group(group);
    }

    fn provide_docs(&self, controller: &dyn SceneController, engine: &Engine) -> Option<String> {
        let game_scene = controller.downcast_ref::<GameScene>()?;
        let scene = &engine.scenes[game_scene.scene];
        self.nodes
            .first()
            .map(|h| scene.graph[*h].doc().to_string())
    }
}

impl PartialEq for GraphSelection {
    fn eq(&self, other: &Self) -> bool {
        utils::is_slice_equal_permutation(self.nodes(), other.nodes())
    }
}

impl GraphSelection {
    pub fn from_list(nodes: Vec<Handle<Node>>) -> Self {
        Self {
            nodes: nodes.into_iter().filter(|h| h.is_some()).collect(),
        }
    }

    /// Creates new selection as single if node handle is not none, and empty if it is.
    pub fn single_or_empty(node: Handle<Node>) -> Self {
        if node.is_none() {
            Self {
                nodes: Default::default(),
            }
        } else {
            Self { nodes: vec![node] }
        }
    }

    /// Adds new selected node, or removes it if it is already in the list of selected nodes.
    pub fn insert_or_exclude(&mut self, handle: Handle<Node>) {
        if let Some(position) = self.nodes.iter().position(|&h| h == handle) {
            self.nodes.remove(position);
        } else {
            self.nodes.push(handle);
        }
    }

    pub fn contains(&self, handle: Handle<Node>) -> bool {
        self.nodes.contains(&handle)
    }

    pub fn nodes(&self) -> &[Handle<Node>] {
        &self.nodes
    }

    pub fn extend(&mut self, other: &GraphSelection) {
        self.nodes.extend_from_slice(&other.nodes)
    }

    pub fn root_nodes(&self, graph: &Graph) -> Vec<Handle<Node>> {
        // Helper function.
        fn is_descendant_of(handle: Handle<Node>, other: Handle<Node>, graph: &Graph) -> bool {
            for &child in graph[other].children() {
                if child == handle {
                    return true;
                }

                let inner = is_descendant_of(handle, child, graph);
                if inner {
                    return true;
                }
            }
            false
        }

        let mut root_nodes = Vec::new();
        for &node in self.nodes().iter() {
            let mut descendant = false;
            for &other_node in self.nodes().iter() {
                if is_descendant_of(node, other_node, graph) {
                    descendant = true;
                    break;
                }
            }
            if !descendant {
                root_nodes.push(node);
            }
        }
        root_nodes
    }

    pub fn global_rotation_position(
        &self,
        graph: &Graph,
    ) -> Option<(UnitQuaternion<f32>, Vector3<f32>)> {
        if self.is_single_selection() {
            if graph.is_valid_handle(self.nodes[0]) {
                Some(graph.global_rotation_position_no_scale(self.nodes[0]))
            } else {
                None
            }
        } else if self.is_empty() {
            None
        } else {
            let mut position = Vector3::default();
            let mut rotation = graph.global_rotation(self.nodes[0]);
            let t = 1.0 / self.nodes.len() as f32;
            for &handle in self.nodes.iter() {
                let global_transform = graph[handle].global_transform();
                position += global_transform.position();
                rotation = rotation.slerp(&graph.global_rotation(self.nodes[0]), t);
            }
            position = position.scale(t);
            Some((rotation, position))
        }
    }

    pub fn offset(&self, graph: &mut Graph, offset: Vector3<f32>) {
        for &handle in self.nodes.iter() {
            let mut chain_scale = Vector3::new(1.0, 1.0, 1.0);
            let mut parent_handle = graph[handle].parent();
            while parent_handle.is_some() {
                let parent = &graph[parent_handle];
                let parent_scale = parent.local_transform().scale();
                chain_scale.x *= parent_scale.x;
                chain_scale.y *= parent_scale.y;
                chain_scale.z *= parent_scale.z;
                parent_handle = parent.parent();
            }

            let offset = Vector3::new(
                if chain_scale.x.abs() > 0.0 {
                    offset.x / chain_scale.x
                } else {
                    offset.x
                },
                if chain_scale.y.abs() > 0.0 {
                    offset.y / chain_scale.y
                } else {
                    offset.y
                },
                if chain_scale.z.abs() > 0.0 {
                    offset.z / chain_scale.z
                } else {
                    offset.z
                },
            );
            graph[handle].local_transform_mut().offset(offset);
        }
    }

    pub fn rotate(&self, graph: &mut Graph, rotation: UnitQuaternion<f32>) {
        for &handle in self.nodes.iter() {
            graph[handle].local_transform_mut().set_rotation(rotation);
        }
    }

    pub fn scale(&self, graph: &mut Graph, scale: Vector3<f32>) {
        for &handle in self.nodes.iter() {
            graph[handle].local_transform_mut().set_scale(scale);
        }
    }

    pub fn local_positions(&self, graph: &Graph) -> Vec<Vector3<f32>> {
        let mut positions = Vec::new();
        for &handle in self.nodes.iter() {
            positions.push(**graph[handle].local_transform().position());
        }
        positions
    }

    pub fn local_rotations(&self, graph: &Graph) -> Vec<UnitQuaternion<f32>> {
        let mut rotations = Vec::new();
        for &handle in self.nodes.iter() {
            rotations.push(**graph[handle].local_transform().rotation());
        }
        rotations
    }

    pub fn local_scales(&self, graph: &Graph) -> Vec<Vector3<f32>> {
        let mut scales = Vec::new();
        for &handle in self.nodes.iter() {
            scales.push(**graph[handle].local_transform().scale());
        }
        scales
    }
}
