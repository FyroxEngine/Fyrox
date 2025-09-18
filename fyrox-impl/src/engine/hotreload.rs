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

use crate::scene::base::NodeMessage;
use crate::{
    asset::manager::ResourceManager,
    core::{
        log::Log,
        pool::{Handle, Ticket},
        visitor::{Visitor, VisitorFlags},
    },
    engine::SerializationContext,
    gui::constructor::WidgetConstructorContainer,
    plugin::Plugin,
    resource::model::ModelResource,
    scene::{
        base::{visit_opt_script, NodeScriptMessage},
        node::Node,
        Scene,
    },
    script::Script,
};
use fyrox_core::visitor::error::VisitError;
use fyrox_core::visitor::VisitAsOption;
use fyrox_graph::BaseSceneGraph;
use std::{
    ops::Deref,
    sync::{mpsc::Sender, Arc},
};

pub struct ScriptState {
    index: usize,
    binary_blob: Vec<u8>,
}

pub struct NodeState {
    node: Handle<Node>,
    ticket: Option<Ticket<Node>>,
    binary_blob: Vec<u8>,
    scripts: Vec<ScriptState>,
}

pub struct SceneState {
    pub scene: Handle<Scene>,
    nodes: Vec<NodeState>,
}

impl SceneState {
    pub fn try_create_from_plugin(
        scene_handle: Handle<Scene>,
        scene: &mut Scene,
        serialization_context: &SerializationContext,
        plugin: &dyn Plugin,
    ) -> Result<Option<Self>, String> {
        let mut scene_state = Self {
            scene: scene_handle,
            nodes: Default::default(),
        };

        for index in 0..scene.graph.capacity() {
            let handle = scene.graph.handle_from_index(index);
            let Ok(node) = scene.graph.try_get_node_mut(handle) else {
                continue;
            };

            let mut node_state = NodeState {
                node: handle,
                ticket: None,
                binary_blob: Default::default(),
                scripts: Default::default(),
            };

            if is_node_belongs_to_plugin(serialization_context, node, plugin) {
                // The entire node belongs to plugin, serialize it entirely.
                // Take the node out of the graph first.
                let (ticket, node) = scene.graph.take_reserve(handle);
                // Using Node::visit_as_option makes the guarantee that writing to the Node is safe (no uninitalized Node (Box dyn NodeTrait) ever appears)
                // even though we only read it here.
                let mut visitor = make_writing_visitor();
                let mut container = Some(node);
                Node::visit_as_option(&mut container, "Node", &mut visitor)
                    .map_err(|e| e.to_string())?;
                node_state.binary_blob = visitor.save_binary_to_vec().map_err(|e| e.to_string())?;
                node_state.ticket = Some(ticket);
            } else {
                // The node does not belong to the plugin, try to check its scripts.
                for (script_index, record) in node.scripts.iter_mut().enumerate() {
                    if let Some(script) = record.script.as_ref() {
                        if is_script_belongs_to_plugin(serialization_context, script, plugin) {
                            // Take the script out of the node and serialize it. The script will be
                            // dropped and destroyed.
                            let mut script = record.script.take();
                            let mut visitor = make_writing_visitor();
                            visit_opt_script("Script", &mut script, &mut visitor)
                                .map_err(|e| e.to_string())?;
                            let binary_blob =
                                visitor.save_binary_to_vec().map_err(|e| e.to_string())?;

                            node_state.scripts.push(ScriptState {
                                index: script_index,
                                binary_blob,
                            })
                        }
                    }
                }
            }

            if !node_state.binary_blob.is_empty() || !node_state.scripts.is_empty() {
                scene_state.nodes.push(node_state);
            }
        }

        if !scene_state.nodes.is_empty() {
            Ok(Some(scene_state))
        } else {
            Ok(None)
        }
    }

    pub fn deserialize_into_scene(
        self,
        scene: &mut Scene,
        serialization_context: &Arc<SerializationContext>,
        resource_manager: &ResourceManager,
        widget_constructors: &Arc<WidgetConstructorContainer>,
    ) -> Result<(), String> {
        // SAFETY: Scene is guaranteed to be used only once per inner loop.
        let scene2 = unsafe { &mut *(scene as *mut Scene) };

        let script_message_sender = scene.graph.script_message_sender.clone();
        let message_sender = scene.graph.message_sender.clone();
        self.deserialize_into_scene_internal(
            |handle: Handle<Node>, index, script| {
                scene.graph[handle].scripts[index].script = script;
            },
            |handle: Handle<Node>, node| {
                scene2.graph[handle] = node;
            },
            script_message_sender,
            message_sender,
            serialization_context,
            resource_manager,
            widget_constructors,
        )
    }

    pub fn deserialize_into_prefab_scene(
        self,
        prefab: &ModelResource,
        serialization_context: &Arc<SerializationContext>,
        resource_manager: &ResourceManager,
        widget_constructors: &Arc<WidgetConstructorContainer>,
    ) -> Result<(), String> {
        let script_message_sender = prefab.data_ref().scene.graph.script_message_sender.clone();
        let message_sender = prefab.data_ref().scene.graph.message_sender.clone();
        self.deserialize_into_scene_internal(
            |handle: Handle<Node>, index, script| {
                prefab.data_ref().scene.graph[handle].scripts[index].script = script;
            },
            |handle: Handle<Node>, node| {
                prefab.data_ref().scene.graph[handle] = node;
            },
            script_message_sender,
            message_sender,
            serialization_context,
            resource_manager,
            widget_constructors,
        )
    }

    pub fn deserialize_into_scene_internal<S, N>(
        self,
        mut set_script: S,
        mut set_node: N,
        script_message_sender: Sender<NodeScriptMessage>,
        message_sender: Sender<NodeMessage>,
        serialization_context: &Arc<SerializationContext>,
        resource_manager: &ResourceManager,
        widget_constructors: &Arc<WidgetConstructorContainer>,
    ) -> Result<(), String>
    where
        S: FnMut(Handle<Node>, usize, Option<Script>),
        N: FnMut(Handle<Node>, Node),
    {
        for node_state in self.nodes {
            if node_state.binary_blob.is_empty() {
                // Only scripts needs to be reloaded.
                for script in node_state.scripts {
                    let mut visitor = make_reading_visitor(
                        &script.binary_blob,
                        serialization_context,
                        resource_manager,
                        widget_constructors,
                    )
                    .map_err(|e| e.to_string())?;
                    let mut opt_script: Option<Script> = None;
                    visit_opt_script("Script", &mut opt_script, &mut visitor)
                        .map_err(|e| e.to_string())?;
                    set_script(node_state.node, script.index, opt_script);

                    Log::info(format!(
                        "Script {} of node {} was successfully deserialized.",
                        script.index, node_state.node
                    ));
                }
            } else {
                let mut visitor = make_reading_visitor(
                    &node_state.binary_blob,
                    serialization_context,
                    resource_manager,
                    widget_constructors,
                )
                .map_err(|e| e.to_string())?;
                // Use visit_as_option to make sure no uninitialized Node (Box<dyn NodeTrait>) ever appears
                let mut container: Option<Node> = None;
                Node::visit_as_option(&mut container, "Node", &mut visitor)
                    .map_err(|e| e.to_string())?;
                if let Some(mut new_node) = container.take() {
                    new_node.on_connected_to_graph(
                        node_state.node,
                        message_sender.clone(),
                        script_message_sender.clone(),
                    );
                    set_node(node_state.node, new_node);

                    Log::info(format!(
                        "Node {} was successfully deserialized.",
                        node_state.node
                    ));
                }
            }
        }

        Ok(())
    }
}

pub fn make_writing_visitor() -> Visitor {
    let mut visitor = Visitor::new();
    visitor.flags = VisitorFlags::SERIALIZE_EVERYTHING;
    visitor
}

pub fn make_reading_visitor(
    binary_blob: &[u8],
    serialization_context: &Arc<SerializationContext>,
    resource_manager: &ResourceManager,
    widget_constructors: &Arc<WidgetConstructorContainer>,
) -> Result<Visitor, VisitError> {
    let mut visitor = Visitor::load_from_memory(binary_blob)?;
    visitor.blackboard.register(serialization_context.clone());
    visitor
        .blackboard
        .register(Arc::new(resource_manager.clone()));
    visitor.blackboard.register(widget_constructors.clone());
    Ok(visitor)
}

fn is_script_belongs_to_plugin(
    serialization_context: &SerializationContext,
    script: &Script,
    plugin: &dyn Plugin,
) -> bool {
    let script_id = script.deref().id();

    if let Some(constructor) = serialization_context
        .script_constructors
        .map()
        .get(&script_id)
    {
        if constructor.assembly_name == plugin.assembly_name() {
            return true;
        }
    }
    false
}

fn is_node_belongs_to_plugin(
    serialization_context: &SerializationContext,
    node: &Node,
    plugin: &dyn Plugin,
) -> bool {
    let node_id = (*node).id();

    if let Some(constructor) = serialization_context.node_constructors.map().get(&node_id) {
        if constructor.assembly_name == plugin.assembly_name() {
            return true;
        }
    }
    false
}
