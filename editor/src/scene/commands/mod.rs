use crate::message::MessageSender;
use crate::scene::clipboard::Clipboard;
use crate::{
    command::Command,
    define_universal_commands,
    scene::{
        clipboard::DeepCloneResult, commands::graph::DeleteSubGraphCommand, EditorScene,
        GraphSelection, Selection,
    },
    Engine, Message,
};
use fyrox::asset::untyped::UntypedResource;
use fyrox::core::variable::mark_inheritable_properties_non_modified;
use fyrox::{
    asset::manager::ResourceManager,
    core::{log::Log, pool::Handle, reflect::prelude::*},
    engine::SerializationContext,
    scene::{graph::SubGraph, node::Node, Scene},
};
use std::any::TypeId;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub mod effect;
pub mod graph;
pub mod material;
pub mod mesh;
pub mod navmesh;
pub mod sound_context;
pub mod terrain;

#[macro_export]
macro_rules! get_set_swap {
    ($self:ident, $host:expr, $get:ident, $set:ident) => {
        match $host {
            host => {
                let old = host.$get();
                let _ = host.$set($self.value.clone());
                $self.value = old;
            }
        }
    };
}

pub struct SceneContext<'a> {
    pub selection: &'a mut Selection,
    pub scene: &'a mut Scene,
    pub scene_content_root: &'a mut Handle<Node>,
    pub clipboard: &'a mut Clipboard,
    pub message_sender: MessageSender,
    pub resource_manager: ResourceManager,
    pub serialization_context: Arc<SerializationContext>,
}

#[derive(Debug)]
pub struct SceneCommand(pub Box<dyn Command>);

impl Deref for SceneCommand {
    type Target = dyn Command;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for SceneCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl SceneCommand {
    pub fn new<C: Command>(cmd: C) -> Self {
        Self(Box::new(cmd))
    }

    pub fn into_inner(self) -> Box<dyn Command> {
        self.0
    }
}

#[derive(Debug)]
pub struct CommandGroup {
    commands: Vec<SceneCommand>,
    custom_name: String,
}

impl From<Vec<SceneCommand>> for CommandGroup {
    fn from(commands: Vec<SceneCommand>) -> Self {
        Self {
            commands,
            custom_name: Default::default(),
        }
    }
}

impl CommandGroup {
    pub fn push(&mut self, command: SceneCommand) {
        self.commands.push(command)
    }

    pub fn with_custom_name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.custom_name = name.as_ref().to_string();
        self
    }
}

impl Command for CommandGroup {
    fn name(&mut self, context: &SceneContext) -> String {
        if self.custom_name.is_empty() {
            let mut name = String::from("Command group: ");
            for cmd in self.commands.iter_mut() {
                name.push_str(&cmd.name(context));
                name.push_str(", ");
            }
            name
        } else {
            self.custom_name.clone()
        }
    }

    fn execute(&mut self, context: &mut SceneContext) {
        for cmd in self.commands.iter_mut() {
            cmd.execute(context);
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        // revert must be done in reverse order.
        for cmd in self.commands.iter_mut().rev() {
            cmd.revert(context);
        }
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        for mut cmd in self.commands.drain(..) {
            cmd.finalize(context);
        }
    }
}

pub fn selection_to_delete(
    editor_selection: &Selection,
    editor_scene: &EditorScene,
) -> GraphSelection {
    // Graph's root is non-deletable.
    let mut selection = if let Selection::Graph(selection) = editor_selection {
        selection.clone()
    } else {
        Default::default()
    };
    if let Some(root_position) = selection
        .nodes
        .iter()
        .position(|&n| n == editor_scene.scene_content_root)
    {
        selection.nodes.remove(root_position);
    }

    selection
}

/// Creates scene command (command group) which removes current selection in editor's scene.
/// This is **not** trivial because each node has multiple connections inside engine and
/// in editor's data model, so we have to thoroughly build command using simple commands.
pub fn make_delete_selection_command(
    editor_selection: &Selection,
    editor_scene: &EditorScene,
    engine: &Engine,
) -> SceneCommand {
    let selection = selection_to_delete(editor_selection, editor_scene);

    let graph = &engine.scenes[editor_scene.scene].graph;

    // Change selection first.
    let mut command_group = CommandGroup::from(vec![SceneCommand::new(
        ChangeSelectionCommand::new(Default::default(), Selection::Graph(selection.clone())),
    )]);

    // Find sub-graphs to delete - we need to do this because we can end up in situation like this:
    // A_
    //   B_      <-
    //   | C       | these are selected
    //   | D_    <-
    //   |   E
    //   F
    // In this case we must deleted only node B, there is no need to delete node D separately because
    // by engine's design when we delete a node, we also delete all its children. So we have to keep
    // this behaviour in editor too.

    let root_nodes = selection.root_nodes(graph);

    for root_node in root_nodes {
        command_group.push(SceneCommand::new(DeleteSubGraphCommand::new(root_node)));
    }

    SceneCommand::new(command_group)
}

#[derive(Debug)]
pub struct ChangeSelectionCommand {
    new_selection: Selection,
    old_selection: Selection,
    cached_name: String,
}

impl ChangeSelectionCommand {
    pub fn new(new_selection: Selection, old_selection: Selection) -> Self {
        Self {
            cached_name: match new_selection {
                Selection::None => "Change Selection: None",
                Selection::Graph(_) => "Change Selection: Graph",
                Selection::Navmesh(_) => "Change Selection: Navmesh",
                Selection::AudioBus(_) => "Change Selection: Audio Bus",
                Selection::Absm(_) => "Change Selection: Absm",
                Selection::Animation(_) => "Change Selection: Animation",
            }
            .to_owned(),
            new_selection,
            old_selection,
        }
    }

    fn swap(&mut self) -> Selection {
        let selection = self.new_selection.clone();
        std::mem::swap(&mut self.new_selection, &mut self.old_selection);
        selection
    }
}

impl Command for ChangeSelectionCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        self.cached_name.clone()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let old_selection = self.old_selection.clone();
        let new_selection = self.swap();
        if &new_selection != context.selection {
            *context.selection = new_selection;
            context
                .message_sender
                .send(Message::SelectionChanged { old_selection });
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let old_selection = self.old_selection.clone();
        let new_selection = self.swap();
        if &new_selection != context.selection {
            *context.selection = new_selection;
            context
                .message_sender
                .send(Message::SelectionChanged { old_selection });
        }
    }
}

#[derive(Debug)]
enum PasteCommandState {
    Undefined,
    NonExecuted,
    Reverted {
        subgraphs: Vec<SubGraph>,
        selection: Selection,
    },
    Executed {
        paste_result: DeepCloneResult,
        last_selection: Selection,
    },
}

#[derive(Debug)]
pub struct PasteCommand {
    parent: Handle<Node>,
    state: PasteCommandState,
}

impl PasteCommand {
    pub fn new(parent: Handle<Node>) -> Self {
        Self {
            parent,
            state: PasteCommandState::NonExecuted,
        }
    }
}

impl Command for PasteCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Paste".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        match std::mem::replace(&mut self.state, PasteCommandState::Undefined) {
            PasteCommandState::NonExecuted => {
                let paste_result = context.clipboard.paste(&mut context.scene.graph);

                for &handle in paste_result.root_nodes.iter() {
                    context.scene.graph.link_nodes(handle, self.parent);
                }

                let mut selection =
                    Selection::Graph(GraphSelection::from_list(paste_result.root_nodes.clone()));
                std::mem::swap(context.selection, &mut selection);

                self.state = PasteCommandState::Executed {
                    paste_result,
                    last_selection: selection,
                };
            }
            PasteCommandState::Reverted {
                subgraphs,
                mut selection,
            } => {
                let mut paste_result = DeepCloneResult {
                    ..Default::default()
                };

                for subgraph in subgraphs {
                    paste_result
                        .root_nodes
                        .push(context.scene.graph.put_sub_graph_back(subgraph));
                }

                for &handle in paste_result.root_nodes.iter() {
                    context.scene.graph.link_nodes(handle, self.parent);
                }

                std::mem::swap(context.selection, &mut selection);
                self.state = PasteCommandState::Executed {
                    paste_result,
                    last_selection: selection,
                };
            }
            _ => unreachable!(),
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        if let PasteCommandState::Executed {
            paste_result,
            mut last_selection,
        } = std::mem::replace(&mut self.state, PasteCommandState::Undefined)
        {
            let mut subgraphs = Vec::new();
            for root_node in paste_result.root_nodes {
                subgraphs.push(context.scene.graph.take_reserve_sub_graph(root_node));
            }

            std::mem::swap(context.selection, &mut last_selection);

            self.state = PasteCommandState::Reverted {
                subgraphs,
                selection: last_selection,
            };
        }
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let PasteCommandState::Reverted { subgraphs, .. } =
            std::mem::replace(&mut self.state, PasteCommandState::Undefined)
        {
            for subgraph in subgraphs {
                context.scene.graph.forget_sub_graph(subgraph);
            }
        }
    }
}

#[derive(Debug)]
pub struct RevertSceneNodePropertyCommand {
    path: String,
    handle: Handle<Node>,
    value: Option<Box<dyn Reflect>>,
}

impl RevertSceneNodePropertyCommand {
    pub fn new(path: String, handle: Handle<Node>) -> Self {
        Self {
            path,
            handle,
            value: None,
        }
    }
}

fn reset_property_modified_flag(entity: &mut dyn Reflect, path: &str) {
    entity.as_reflect_mut(&mut |entity| {
        entity.resolve_path_mut(path, &mut |result| {
            mark_inheritable_properties_non_modified(
                result.unwrap(),
                &[TypeId::of::<UntypedResource>()],
            );
        })
    })
}

impl Command for RevertSceneNodePropertyCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        format!("Revert {} Property", self.path)
    }

    fn execute(&mut self, context: &mut SceneContext) {
        let child = &mut context.scene.graph[self.handle];

        // Revert only if there's parent resource (the node is an instance of some resource).
        if let Some(resource) = child.resource().as_ref() {
            let resource_data = resource.data_ref();
            let parent = &resource_data.get_scene().graph[child.original_handle_in_resource()];

            let mut parent_value = None;

            // Find and clone parent's value first.
            parent.as_reflect(&mut |parent| {
                parent.resolve_path(&self.path, &mut |result| match result {
                    Ok(parent_field) => parent_field.as_inheritable_variable(&mut |parent_field| {
                        if let Some(parent_inheritable) = parent_field {
                            parent_value = Some(parent_inheritable.clone_value_box());
                        }
                    }),
                    Err(e) => Log::err(format!(
                        "Failed to resolve parent path {}. Reason: {:?}",
                        self.path, e
                    )),
                })
            });

            // Check whether the child's field is inheritable and modified.
            let mut need_revert = false;

            child.as_reflect_mut(&mut |child| {
                child.resolve_path_mut(&self.path, &mut |result| match result {
                    Ok(child_field) => {
                        child_field.as_inheritable_variable_mut(&mut |child_inheritable| {
                            if let Some(child_inheritable) = child_inheritable {
                                need_revert = child_inheritable.is_modified();
                            } else {
                                Log::err(format!("Property {} is not inheritable!", self.path))
                            }
                        })
                    }
                    Err(e) => Log::err(format!(
                        "Failed to resolve child path {}. Reason: {:?}",
                        self.path, e
                    )),
                });
            });

            // Try to apply it to the child.
            if need_revert {
                if let Some(parent_value) = parent_value {
                    let mut was_set = false;

                    let mut parent_value = Some(parent_value);
                    child.as_reflect_mut(&mut |child| {
                        child.set_field_by_path(
                            &self.path,
                            parent_value.take().unwrap(),
                            &mut |result| match result {
                                Ok(old_value) => {
                                    self.value = Some(old_value);

                                    was_set = true;
                                }
                                Err(_) => Log::err(format!(
                                    "Failed to revert property {}. Reason: no such property!",
                                    self.path
                                )),
                            },
                        );
                    });

                    if was_set {
                        // Reset modified flag.
                        reset_property_modified_flag(child, &self.path);
                    }
                }
            }
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        // If the property was modified, then simply set it to previous value to make it modified again.
        if let Some(old_value) = self.value.take() {
            let mut old_value = Some(old_value);
            context.scene.graph[self.handle].as_reflect_mut(&mut |node| {
                node.set_field_by_path(&self.path, old_value.take().unwrap(), &mut |result| {
                    if result.is_err() {
                        Log::err(format!(
                            "Failed to revert property {}. Reason: no such property!",
                            self.path
                        ))
                    }
                });
            })
        }
    }
}

define_universal_commands!(
    make_set_node_property_command,
    Command,
    SceneCommand,
    SceneContext,
    Handle<Node>,
    ctx,
    handle,
    self,
    { &mut ctx.scene.graph[self.handle] as &mut dyn Reflect },
);
