use crate::{
    command::GameSceneCommandTrait,
    define_universal_commands,
    message::MessageSender,
    scene::{
        clipboard::{Clipboard, DeepCloneResult},
        commands::graph::DeleteSubGraphCommand,
        GameScene, GraphSelection, Selection,
    },
    Engine, Message,
};
use fyrox::{
    asset::manager::ResourceManager,
    core::{log::Log, pool::Handle, reflect::prelude::*},
    engine::SerializationContext,
    graph::{SceneGraph, SceneGraphNode},
    scene::{graph::SubGraph, node::Node, Scene},
};
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

pub struct GameSceneContext<'a> {
    pub selection: &'a mut Selection,
    pub scene: &'a mut Scene,
    pub scene_content_root: &'a mut Handle<Node>,
    pub clipboard: &'a mut Clipboard,
    pub message_sender: MessageSender,
    pub resource_manager: ResourceManager,
    pub serialization_context: Arc<SerializationContext>,
}

#[derive(Debug)]
pub struct GameSceneCommand(pub Box<dyn GameSceneCommandTrait>);

impl Deref for GameSceneCommand {
    type Target = dyn GameSceneCommandTrait;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for GameSceneCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl GameSceneCommand {
    pub fn new<C: GameSceneCommandTrait>(cmd: C) -> Self {
        Self(Box::new(cmd))
    }

    pub fn into_inner(self) -> Box<dyn GameSceneCommandTrait> {
        self.0
    }
}

#[derive(Debug, Default)]
pub struct CommandGroup {
    commands: Vec<GameSceneCommand>,
    custom_name: String,
}

impl From<Vec<GameSceneCommand>> for CommandGroup {
    fn from(commands: Vec<GameSceneCommand>) -> Self {
        Self {
            commands,
            custom_name: Default::default(),
        }
    }
}

impl CommandGroup {
    pub fn push<C: GameSceneCommandTrait>(&mut self, command: C) {
        self.commands.push(GameSceneCommand::new(command))
    }

    pub fn with_custom_name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.custom_name = name.as_ref().to_string();
        self
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

impl GameSceneCommandTrait for CommandGroup {
    fn name(&mut self, context: &GameSceneContext) -> String {
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

    fn execute(&mut self, context: &mut GameSceneContext) {
        for cmd in self.commands.iter_mut() {
            cmd.execute(context);
        }
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        // revert must be done in reverse order.
        for cmd in self.commands.iter_mut().rev() {
            cmd.revert(context);
        }
    }

    fn finalize(&mut self, context: &mut GameSceneContext) {
        for mut cmd in self.commands.drain(..) {
            cmd.finalize(context);
        }
    }
}

pub fn selection_to_delete(editor_selection: &Selection, game_scene: &GameScene) -> GraphSelection {
    // Graph's root is non-deletable.
    let mut selection = if let Some(selection) = editor_selection.as_graph() {
        selection.clone()
    } else {
        Default::default()
    };
    if let Some(root_position) = selection
        .nodes
        .iter()
        .position(|&n| n == game_scene.scene_content_root)
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
    game_scene: &GameScene,
    engine: &Engine,
) -> GameSceneCommand {
    let selection = selection_to_delete(editor_selection, game_scene);

    let graph = &engine.scenes[game_scene.scene].graph;

    // Change selection first.
    let mut command_group = CommandGroup::from(vec![GameSceneCommand::new(
        ChangeSelectionCommand::new(Default::default(), Selection::new(selection.clone())),
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
        command_group.push(DeleteSubGraphCommand::new(root_node));
    }

    GameSceneCommand::new(command_group)
}

#[derive(Debug)]
pub struct ChangeSelectionCommand {
    new_selection: Selection,
    old_selection: Selection,
}

impl ChangeSelectionCommand {
    pub fn new(new_selection: Selection, old_selection: Selection) -> Self {
        Self {
            new_selection,
            old_selection,
        }
    }

    fn swap(&mut self) -> Selection {
        let selection = self.new_selection.clone();
        std::mem::swap(&mut self.new_selection, &mut self.old_selection);
        selection
    }

    fn exec(&mut self, context: &mut GameSceneContext) {
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

impl GameSceneCommandTrait for ChangeSelectionCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Change Selection".to_string()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        self.exec(context);
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        self.exec(context);
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

impl GameSceneCommandTrait for PasteCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Paste".to_owned()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        match std::mem::replace(&mut self.state, PasteCommandState::Undefined) {
            PasteCommandState::NonExecuted => {
                let paste_result = context.clipboard.paste(&mut context.scene.graph);

                for &handle in paste_result.root_nodes.iter() {
                    context.scene.graph.link_nodes(handle, self.parent);
                }

                let mut selection =
                    Selection::new(GraphSelection::from_list(paste_result.root_nodes.clone()));
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

    fn revert(&mut self, context: &mut GameSceneContext) {
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

    fn finalize(&mut self, context: &mut GameSceneContext) {
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

impl GameSceneCommandTrait for RevertSceneNodePropertyCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        format!("Revert {} Property", self.path)
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        let child = &mut context.scene.graph[self.handle];
        self.value = child.revert_inheritable_property(&self.path);
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
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
    GameSceneCommandTrait,
    GameSceneCommand,
    GameSceneContext,
    Handle<Node>,
    ctx,
    handle,
    self,
    { &mut ctx.scene.graph[self.handle] as &mut dyn Reflect },
);
