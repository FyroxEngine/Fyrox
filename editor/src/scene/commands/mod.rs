use crate::{
    command::Command,
    scene::{
        clipboard::DeepCloneResult, commands::graph::DeleteSubGraphCommand, EditorScene,
        GraphSelection, Selection,
    },
    GameEngine, Message,
};
use fyrox::{
    core::{
        pool::Handle,
        reflect::{self, Component, Reflect, ResolvePath},
    },
    engine::{resource_manager::ResourceManager, SerializationContext},
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{graph::SubGraph, node::Node, Scene},
    utils::log::Log,
};
use std::{
    ops::{Deref, DerefMut},
    sync::{mpsc::Sender, Arc},
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
    pub editor_scene: &'a mut EditorScene,
    pub scene: &'a mut Scene,
    pub message_sender: Sender<Message>,
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
                Selection::SoundContext => "Change Selection: Sound Context",
                Selection::Effect(_) => "Change Selection: Effect",
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
                    .paste(&mut context.scene.graph);

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

            std::mem::swap(&mut context.editor_scene.selection, &mut last_selection);

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

#[macro_export]
macro_rules! define_node_command {
    ($($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $node:ident) $apply_method:block )*) => {
        $(
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
        )*
    };
}

#[macro_export]
macro_rules! define_swap_command {
    // core impl
    ($(#[$meta:meta])* $type:ident($value_type:ty): $name:expr, $swap:expr) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $type {
            handle: fyrox::core::pool::Handle<Node>,
            value: $value_type,
        }

        impl $type {
            pub fn new(handle: fyrox::core::pool::Handle<Node>, value: $value_type) -> Self {
                Self { handle, value }
            }

            fn swap(&mut self, graph: &mut fyrox::scene::graph::Graph) {
                #[allow(clippy::redundant_closure_call)]
                ($swap)(self, graph)
            }
        }

        impl Command for $type {
            fn name(&mut self, _context: &SceneContext) -> String {
                $name.to_owned()
            }

            fn execute(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.scene.graph);
            }

            fn revert(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.scene.graph);
            }
        }
    };

    // cast `&mut Node` and use their setter/getter methods for swapping them
    ($cast_node:expr, $(
        $(#[$meta:meta])* $type:ident($value_type:ty): $get:ident, $set:ident, $name:expr;
     )*) => {
        $(
            $crate::define_swap_command! {
                $(#[$meta:meta])*
                $type($value_type):
                $name, |me: &mut $type, graph: &mut fyrox::scene::graph::Graph| {
                    let mut node = &mut graph[me.handle];
                    let host = ($cast_node)(&mut node);
                    let old = host.$get();
                    let _ = host.$set(me.value.clone());
                    me.value = old;
                }
            }
        )+
    };
}

enum Action {
    Modify { value: Box<dyn Reflect> },
    AddItem { value: Box<dyn Reflect> },
    RemoveItem { index: usize },
}

impl Action {
    fn from_field_kind(field_kind: &FieldKind) -> Self {
        match field_kind {
            FieldKind::Object(ref value) => Self::Modify {
                value: value.clone().into_box_reflect(),
            },
            FieldKind::Collection(ref collection_changed) => match **collection_changed {
                CollectionChanged::Add(ref value) => Self::AddItem {
                    value: value.clone().into_box_reflect(),
                },
                CollectionChanged::Remove(index) => Self::RemoveItem { index },
                CollectionChanged::ItemChanged { ref property, .. } => {
                    Self::from_field_kind(&property.value)
                }
            },
            FieldKind::Inspectable(ref inspectable) => Self::from_field_kind(&inspectable.value),
        }
    }
}

pub fn make_set_node_property_command(
    node: Handle<Node>,
    property_changed: &PropertyChanged,
) -> SceneCommand {
    match Action::from_field_kind(&property_changed.value) {
        Action::Modify { value } => SceneCommand::new(SetNodePropertyCommand::new(
            node,
            property_changed.path(),
            value,
        )),
        Action::AddItem { value } => SceneCommand::new(AddNodeCollectionItemCommand::new(
            node,
            property_changed.path(),
            value,
        )),
        Action::RemoveItem { index } => SceneCommand::new(RemoveNodeCollectionItemCommand::new(
            node,
            property_changed.path(),
            index,
        )),
    }
}

fn try_modify_property<F: FnOnce(&mut dyn Reflect)>(
    handle: Handle<Node>,
    context: &mut SceneContext,
    path: &str,
    func: F,
) {
    match context.scene.graph[handle]
        .as_reflect_mut()
        .resolve_path_mut(path)
    {
        Ok(field) => func(field),
        Err(e) => Log::err(format!(
            "There is no such property {}! Reason: {:?}",
            path, e
        )),
    }
}

#[derive(Debug)]
pub struct SetNodePropertyCommand {
    node: Handle<Node>,
    value: Option<Box<dyn Reflect>>,
    path: String,
}

impl SetNodePropertyCommand {
    pub fn new(node: Handle<Node>, path: String, value: Box<dyn Reflect>) -> Self {
        Self {
            node,
            value: Some(value),
            path,
        }
    }

    fn swap(&mut self, context: &mut SceneContext) {
        let mut components = reflect::path_to_components(&self.path);
        if let Some(Component::Field(field)) = components.pop() {
            let mut parent_path = String::new();
            for component in components.into_iter() {
                match component {
                    Component::Field(s) => {
                        if !parent_path.is_empty() {
                            parent_path.push('.');
                        }
                        parent_path += s;
                    }
                    Component::Index(s) => {
                        parent_path.push('[');
                        parent_path += s;
                        parent_path.push(']');
                    }
                }
            }

            let node = &mut context.scene.graph[self.node];
            let parent_entity = if parent_path.is_empty() {
                node.as_reflect_mut()
            } else {
                match node.as_reflect_mut().resolve_path_mut(&parent_path) {
                    Err(e) => {
                        Log::err(format!(
                            "There is no such parent property {}! Reason: {:?}",
                            parent_path, e
                        ));

                        return;
                    }
                    Ok(property) => property,
                }
            };

            match parent_entity.set_field(field, self.value.take().unwrap()) {
                Ok(old_value) => {
                    self.value = Some(old_value);
                }
                Err(current_value) => {
                    self.value = Some(current_value);
                    Log::err(format!(
                        "Failed to set property {}! Incompatible types!",
                        self.path
                    ))
                }
            }
        } else {
            Log::err(format!(
                "Failed to set property {}! Invalid path!",
                self.path
            ))
        }
    }
}

impl Command for SetNodePropertyCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        format!("Set node {} property", self.path)
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }
}

#[derive(Debug)]
pub struct AddNodeCollectionItemCommand {
    node: Handle<Node>,
    path: String,
    item: Option<Box<dyn Reflect>>,
}

impl AddNodeCollectionItemCommand {
    pub fn new(node: Handle<Node>, path: String, item: Box<dyn Reflect>) -> Self {
        Self {
            node,
            path,
            item: Some(item),
        }
    }
}

impl Command for AddNodeCollectionItemCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        format!("Add item to {} collection", self.path)
    }

    fn execute(&mut self, context: &mut SceneContext) {
        try_modify_property(self.node, context, &self.path, |field| {
            if let Some(list) = field.as_list_mut() {
                if let Err(item) = list.reflect_push(self.item.take().unwrap()) {
                    self.item = Some(item);
                    Log::err(format!(
                        "Failed to push item to {} collection. Type mismatch!",
                        self.path
                    ))
                }
            } else {
                Log::err(format!("Property {} is not a collection!", self.path))
            }
        })
    }

    fn revert(&mut self, context: &mut SceneContext) {
        try_modify_property(self.node, context, &self.path, |field| {
            if let Some(list) = field.as_list_mut() {
                if let Some(item) = list.reflect_pop() {
                    self.item = Some(item);
                } else {
                    Log::err(format!("Failed to pop item from {} collection!", self.path))
                }
            } else {
                Log::err(format!("Property {} is not a collection!", self.path))
            }
        })
    }
}

#[derive(Debug)]
pub struct RemoveNodeCollectionItemCommand {
    node: Handle<Node>,
    path: String,
    index: usize,
    value: Option<Box<dyn Reflect>>,
}

impl RemoveNodeCollectionItemCommand {
    pub fn new(node: Handle<Node>, path: String, index: usize) -> Self {
        Self {
            node,
            path,
            index,
            value: None,
        }
    }
}

impl Command for RemoveNodeCollectionItemCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        format!("Remove node collection {} item {}", self.path, self.index)
    }

    fn execute(&mut self, context: &mut SceneContext) {
        try_modify_property(self.node, context, &self.path, |field| {
            if let Some(list) = field.as_list_mut() {
                self.value = list.reflect_remove(self.index);
            } else {
                Log::err(format!("Property {} is not a collection!", self.path))
            }
        })
    }

    fn revert(&mut self, context: &mut SceneContext) {
        try_modify_property(self.node, context, &self.path, |field| {
            if let Some(list) = field.as_list_mut() {
                if let Err(item) = list.reflect_insert(self.index, self.value.take().unwrap()) {
                    self.value = Some(item);
                } else {
                    Log::err(format!(
                        "Failed to insert item to {} collection. Type mismatch!",
                        self.path
                    ))
                }
            } else {
                Log::err(format!("Property {} is not a collection!", self.path))
            }
        })
    }
}
