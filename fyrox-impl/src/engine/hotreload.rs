use crate::{
    asset::manager::ResourceManager,
    core::{
        log::Log,
        pool::{Handle, PayloadContainer, Ticket},
        visitor::{Visit, VisitError, Visitor, VisitorFlags},
    },
    engine::SerializationContext,
    gui::constructor::WidgetConstructorContainer,
    plugin::Plugin,
    scene::{
        base::visit_opt_script,
        node::{container::NodeContainer, Node},
        Scene,
    },
    script::Script,
};
use std::{ops::Deref, sync::Arc};

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
            let Some(node) = scene.graph.try_get_mut(handle) else {
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
                let mut container = NodeContainer::new(node);
                let mut visitor = make_writing_visitor();
                container
                    .visit("Node", &mut visitor)
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
        let script_message_sender = scene.graph.script_message_sender.clone();

        for node_state in self.nodes {
            let node = &mut scene.graph[node_state.node];

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
                    node.scripts[script.index].script = opt_script;

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
                let mut container = NodeContainer::default();
                container
                    .visit("Node", &mut visitor)
                    .map_err(|e| e.to_string())?;
                if let Some(mut new_node) = container.take() {
                    new_node.script_message_sender = Some(script_message_sender.clone());
                    *node = new_node;

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
