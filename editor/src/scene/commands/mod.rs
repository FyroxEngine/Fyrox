use crate::fyrox::{
    asset::manager::ResourceManager,
    core::{log::Log, pool::Handle, reflect::prelude::*, type_traits::prelude::*},
    engine::SerializationContext,
    graph::{BaseSceneGraph, SceneGraphNode},
    scene::{graph::SubGraph, node::Node, Scene},
};
use crate::{
    command::{Command, CommandContext, CommandGroup, CommandTrait},
    message::MessageSender,
    scene::{
        clipboard::{Clipboard, DeepCloneResult},
        commands::graph::DeleteSubGraphCommand,
        GameScene, GraphSelection, Selection,
    },
    Engine, Message,
};
use std::sync::Arc;

pub mod effect;
pub mod graph;
pub mod material;
pub mod mesh;
pub mod navmesh;
pub mod sound_context;
pub mod terrain;

#[derive(ComponentProvider)]
pub struct GameSceneContext {
    #[component(include)]
    pub selection: &'static mut Selection,
    pub scene: &'static mut Scene,
    pub scene_content_root: &'static mut Handle<Node>,
    pub clipboard: &'static mut Clipboard,
    #[component(include)]
    pub message_sender: MessageSender,
    pub resource_manager: ResourceManager,
    pub serialization_context: Arc<SerializationContext>,
}

impl GameSceneContext {
    pub fn exec<'a, F>(
        selection: &'a mut Selection,
        scene: &'a mut Scene,
        scene_content_root: &'a mut Handle<Node>,
        clipboard: &'a mut Clipboard,
        message_sender: MessageSender,
        resource_manager: ResourceManager,
        serialization_context: Arc<SerializationContext>,
        func: F,
    ) where
        F: FnOnce(&mut GameSceneContext),
    {
        // SAFETY: Temporarily extend lifetime to 'static and execute external closure with it.
        // The closure accepts this extended context by reference, so there's no way it escapes to
        // outer world. The initial lifetime is still preserved by this function call.
        func(unsafe {
            &mut Self {
                selection: std::mem::transmute::<&'a mut _, &'static mut _>(selection),
                scene: std::mem::transmute::<&'a mut _, &'static mut _>(scene),
                scene_content_root: std::mem::transmute::<&'a mut _, &'static mut _>(
                    scene_content_root,
                ),
                clipboard: std::mem::transmute::<&'a mut _, &'static mut _>(clipboard),
                message_sender,
                resource_manager,
                serialization_context,
            }
        });
    }
}

impl CommandContext for GameSceneContext {}

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
) -> Command {
    let selection = selection_to_delete(editor_selection, game_scene);

    let graph = &engine.scenes[game_scene.scene].graph;

    // Change selection first.
    let mut command_group = CommandGroup::from(vec![Command::new(ChangeSelectionCommand::new(
        Default::default(),
    ))]);

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

    Command::new(command_group)
}

#[derive(Debug)]
pub struct ChangeSelectionCommand {
    new_selection: Selection,
}

impl ChangeSelectionCommand {
    pub fn new(new_selection: Selection) -> Self {
        Self { new_selection }
    }

    fn exec(&mut self, context: &mut dyn CommandContext) {
        let current_selection = context.get_mut::<&mut Selection>();

        if &self.new_selection != *current_selection {
            let old_selection = current_selection.clone();

            std::mem::swap(*current_selection, &mut self.new_selection);

            context
                .get::<MessageSender>()
                .send(Message::SelectionChanged { old_selection });
        }
    }
}

impl CommandTrait for ChangeSelectionCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Change Selection".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.exec(context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
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

impl CommandTrait for PasteCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Paste".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

impl CommandTrait for RevertSceneNodePropertyCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        format!("Revert {} Property", self.path)
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let child = &mut context.scene.graph[self.handle];
        self.value = child.revert_inheritable_property(&self.path);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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
