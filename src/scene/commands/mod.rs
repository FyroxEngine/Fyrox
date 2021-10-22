use crate::scene::commands::physics::{SetJointBody1Command, SetJointBody2Command};
use crate::{
    command::Command,
    physics::{Collider, Joint, RigidBody},
    scene::{
        clipboard::DeepCloneResult,
        commands::{
            graph::DeleteSubGraphCommand,
            physics::{DeleteBodyCommand, DeleteColliderCommand, DeleteJointCommand},
        },
        EditorScene, GraphSelection, Selection,
    },
    GameEngine, Message,
};
use rg3d::{
    core::pool::{ErasedHandle, Handle, Ticket},
    engine::resource_manager::ResourceManager,
    scene::{graph::SubGraph, node::Node, Scene},
};
use std::ops::{Deref, DerefMut};
use std::{collections::HashMap, sync::mpsc::Sender};

pub mod camera;
pub mod decal;
pub mod graph;
pub mod light;
pub mod lod;
pub mod material;
pub mod mesh;
pub mod navmesh;
pub mod particle_system;
pub mod physics;
pub mod sound;
pub mod sprite;
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
    pub editor_scene: &'a mut EditorScene,
    pub scene: &'a mut Scene,
    pub message_sender: Sender<Message>,
    pub resource_manager: ResourceManager,
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
}

impl From<Vec<SceneCommand>> for CommandGroup {
    fn from(commands: Vec<SceneCommand>) -> Self {
        Self { commands }
    }
}

impl CommandGroup {
    pub fn push(&mut self, command: SceneCommand) {
        self.commands.push(command)
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Command for CommandGroup {
    fn name(&mut self, context: &SceneContext) -> String {
        let mut name = String::from("Command group: ");
        for cmd in self.commands.iter_mut() {
            name.push_str(&cmd.name(context));
            name.push_str(", ");
        }
        name
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

/// Creates scene command (command group) which removes current selection in editor's scene.
/// This is **not** trivial because each node has multiple connections inside engine and
/// in editor's data model, so we have to thoroughly build command using simple commands.
pub fn make_delete_selection_command(
    editor_scene: &EditorScene,
    engine: &GameEngine,
) -> SceneCommand {
    let graph = &engine.scenes[editor_scene.scene].graph;

    // Graph's root is non-deletable.
    let mut selection = if let Selection::Graph(selection) = &editor_scene.selection {
        selection.clone()
    } else {
        Default::default()
    };
    if let Some(root_position) = selection.nodes.iter().position(|&n| n == graph.get_root()) {
        selection.nodes.remove(root_position);
    }

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

    // Delete all associated physics entities in the whole hierarchy starting from root nodes
    // found above.
    let mut stack = root_nodes.clone();
    while let Some(node) = stack.pop() {
        if let Some(&body) = editor_scene.physics.binder.value_of(&node) {
            for &collider in editor_scene.physics.bodies[body].colliders.iter() {
                command_group.push(SceneCommand::new(DeleteColliderCommand::new(
                    collider.into(),
                )))
            }

            command_group.push(SceneCommand::new(DeleteBodyCommand::new(body)));

            // Remove any associated joints.
            let joint = editor_scene.physics.find_joint(body);
            if joint.is_some() {
                command_group.push(SceneCommand::new(DeleteJointCommand::new(joint)));
            }

            // Also check if this node is attached to a joint.
            for (handle, joint) in editor_scene.physics.joints.pair_iter() {
                if joint.body1 == ErasedHandle::from(body) {
                    command_group.push(SceneCommand::new(SetJointBody1Command::new(
                        handle,
                        ErasedHandle::none(),
                    )));
                } else if joint.body2 == ErasedHandle::from(body) {
                    command_group.push(SceneCommand::new(SetJointBody2Command::new(
                        handle,
                        ErasedHandle::none(),
                    )));
                }
            }
        }
        stack.extend_from_slice(graph[node].children());
    }

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
                Selection::Sound(_) => "Change Selection: Sound",
                Selection::RigidBody(_) => "Change Selection: RigidBody",
                Selection::Joint(_) => "Change Selection: Joint",
                Selection::Collider(_) => "Change Selection: Collider",
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
        let new_selection = self.swap();
        if new_selection != context.editor_scene.selection {
            context.editor_scene.selection = new_selection;
            context
                .message_sender
                .send(Message::SelectionChanged)
                .unwrap();
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let new_selection = self.swap();
        if new_selection != context.editor_scene.selection {
            context.editor_scene.selection = new_selection;
            context
                .message_sender
                .send(Message::SelectionChanged)
                .unwrap();
        }
    }
}

#[derive(Debug)]
enum PasteCommandState {
    Undefined,
    NonExecuted,
    Reverted {
        subgraphs: Vec<SubGraph>,
        bodies: Vec<(Ticket<RigidBody>, RigidBody)>,
        colliders: Vec<(Ticket<Collider>, Collider)>,
        joints: Vec<(Ticket<Joint>, Joint)>,
        binder: HashMap<Handle<Node>, Handle<RigidBody>>,
        selection: Selection,
    },
    Executed {
        paste_result: DeepCloneResult,
        last_selection: Selection,
    },
}

#[derive(Debug)]
pub struct PasteCommand {
    state: PasteCommandState,
}

impl Default for PasteCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl PasteCommand {
    pub fn new() -> Self {
        Self {
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
                let paste_result = context
                    .editor_scene
                    .clipboard
                    .paste(&mut context.scene.graph, &mut context.editor_scene.physics);

                let mut selection =
                    Selection::Graph(GraphSelection::from_list(paste_result.root_nodes.clone()));
                std::mem::swap(&mut context.editor_scene.selection, &mut selection);

                self.state = PasteCommandState::Executed {
                    paste_result,
                    last_selection: selection,
                };
            }
            PasteCommandState::Reverted {
                subgraphs,
                bodies,
                colliders,
                joints,
                binder,
                mut selection,
            } => {
                let mut paste_result = DeepCloneResult {
                    binder,
                    ..Default::default()
                };

                for subgraph in subgraphs {
                    paste_result
                        .root_nodes
                        .push(context.scene.graph.put_sub_graph_back(subgraph));
                }

                for (ticket, body) in bodies {
                    paste_result
                        .bodies
                        .push(context.editor_scene.physics.bodies.put_back(ticket, body));
                }

                for (ticket, collider) in colliders {
                    paste_result.colliders.push(
                        context
                            .editor_scene
                            .physics
                            .colliders
                            .put_back(ticket, collider),
                    );
                }

                for (ticket, joint) in joints {
                    paste_result
                        .joints
                        .push(context.editor_scene.physics.joints.put_back(ticket, joint));
                }

                for (&node, &body) in paste_result.binder.iter() {
                    context.editor_scene.physics.binder.insert(node, body);
                }

                std::mem::swap(&mut context.editor_scene.selection, &mut selection);
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

            let mut bodies = Vec::new();
            for body in paste_result.bodies {
                bodies.push(context.editor_scene.physics.bodies.take_reserve(body));
            }

            let mut colliders = Vec::new();
            for collider in paste_result.colliders {
                colliders.push(
                    context
                        .editor_scene
                        .physics
                        .colliders
                        .take_reserve(collider),
                );
            }

            let mut joints = Vec::new();
            for joint in paste_result.joints {
                joints.push(context.editor_scene.physics.joints.take_reserve(joint));
            }

            for (node, _) in paste_result.binder.iter() {
                context.editor_scene.physics.binder.remove_by_key(node);
            }

            std::mem::swap(&mut context.editor_scene.selection, &mut last_selection);

            self.state = PasteCommandState::Reverted {
                subgraphs,
                bodies,
                colliders,
                joints,
                binder: paste_result.binder,
                selection: last_selection,
            };
        }
    }

    fn finalize(&mut self, context: &mut SceneContext) {
        if let PasteCommandState::Reverted {
            subgraphs,
            bodies,
            colliders,
            joints,
            ..
        } = std::mem::replace(&mut self.state, PasteCommandState::Undefined)
        {
            for subgraph in subgraphs {
                context.scene.graph.forget_sub_graph(subgraph);
            }

            for (ticket, _) in bodies {
                context.editor_scene.physics.bodies.forget_ticket(ticket);
            }

            for (ticket, _) in colliders {
                context.editor_scene.physics.colliders.forget_ticket(ticket)
            }

            for (ticket, _) in joints {
                context.editor_scene.physics.joints.forget_ticket(ticket);
            }
        }
    }
}

#[macro_export]
macro_rules! define_node_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $node:ident) $apply_method:block ) => {
        #[derive(Debug)]
        pub struct $name {
            handle: Handle<Node>,
            value: $value_type,
        }

        impl $name {
            pub fn new(handle: Handle<Node>, value: $value_type) -> Self {
                Self { handle, value }
            }

            fn swap(&mut $self, graph: &mut Graph) {
                let $node = &mut graph[$self.handle];
                $apply_method
            }
        }

        impl Command for $name {
            fn name(&mut self, _context: &SceneContext) -> String {
                $human_readable_name.to_owned()
            }

            fn execute(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.scene.graph);
            }

            fn revert(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.scene.graph);
            }
        }
    };
}
