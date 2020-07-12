use crate::{command::Command, Message};
use rg3d::{
    core::{
        math::{quat::Quat, vec3::Vec3},
        pool::{Handle, Ticket},
    },
    scene::{graph::Graph, node::Node, Scene},
};
use std::sync::mpsc::Sender;

pub struct EditorScene {
    pub scene: Handle<Scene>,
    // Handle to a root for all editor nodes.
    pub root: Handle<Node>,
}

#[derive(Debug)]
pub enum SceneCommand {
    CommandGroup(Vec<SceneCommand>),
    AddNode(AddNodeCommand),
    DeleteNode(DeleteNodeCommand),
    ChangeSelection(ChangeSelectionCommand),
    MoveNode(MoveNodeCommand),
    ScaleNode(ScaleNodeCommand),
    RotateNode(RotateNodeCommand),
    LinkNodes(LinkNodesCommand),
    SetVisible(SetVisibleCommand),
}

pub struct MutRefLt<'a, T: 'a> {
    field: &'a mut T,
}

pub struct SceneContext<'a> {
    pub graph: &'a mut Graph,
    pub message_sender: Sender<Message>,
    pub current_selection: Handle<Node>,
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            SceneCommand::CommandGroup(v) => {
                for cmd in v {
                    cmd.$func($($args),*)
                }
            },
            SceneCommand::AddNode(v) => v.$func($($args),*),
            SceneCommand::DeleteNode(v) => v.$func($($args),*),
            SceneCommand::ChangeSelection(v) => v.$func($($args),*),
            SceneCommand::MoveNode(v) => v.$func($($args),*),
            SceneCommand::ScaleNode(v) => v.$func($($args),*),
            SceneCommand::RotateNode(v) => v.$func($($args),*),
            SceneCommand::LinkNodes(v) => v.$func($($args),*),
            SceneCommand::SetVisible(v) => v.$func($($args),*),
        }
    };
}

impl<'a> Command<'a> for SceneCommand {
    type Context = SceneContext<'a>;

    fn execute(&mut self, context: &mut Self::Context) {
        static_dispatch!(self, execute, context);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        static_dispatch!(self, revert, context);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        static_dispatch!(self, finalize, context);
    }
}

#[derive(Debug)]
pub struct AddNodeCommand {
    ticket: Option<Ticket<Node>>,
    handle: Handle<Node>,
    node: Option<Node>,
}

impl AddNodeCommand {
    pub fn new(node: Node) -> Self {
        Self {
            ticket: None,
            handle: Default::default(),
            node: Some(node),
        }
    }
}

impl<'a> Command<'a> for AddNodeCommand {
    type Context = SceneContext<'a>;

    fn execute(&mut self, context: &mut Self::Context) {
        match self.ticket.take() {
            None => {
                self.handle = context.graph.add_node(self.node.take().unwrap());
            }
            Some(ticket) => {
                context.graph.put_back(ticket, self.node.take().unwrap());
            }
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let (ticket, node) = context.graph.take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.node = Some(node);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.graph.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct ChangeSelectionCommand {
    new_selection: Handle<Node>,
    old_selection: Handle<Node>,
}

impl ChangeSelectionCommand {
    pub fn new(new_selection: Handle<Node>, old_selection: Handle<Node>) -> Self {
        Self {
            new_selection,
            old_selection,
        }
    }

    fn swap(&mut self) -> Handle<Node> {
        let selection = self.new_selection;
        std::mem::swap(&mut self.new_selection, &mut self.old_selection);
        selection
    }
}

impl<'a> Command<'a> for ChangeSelectionCommand {
    type Context = SceneContext<'a>;

    fn execute(&mut self, context: &mut Self::Context) {
        let new_selection = self.swap();
        if new_selection != context.current_selection {
            context
                .message_sender
                .send(Message::SetSelection(new_selection))
                .unwrap();
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let new_selection = self.swap();
        if new_selection != context.current_selection {
            context
                .message_sender
                .send(Message::SetSelection(new_selection))
                .unwrap();
        }
    }
}

#[derive(Debug)]
pub struct MoveNodeCommand {
    node: Handle<Node>,
    old_position: Vec3,
    new_position: Vec3,
}

impl MoveNodeCommand {
    pub fn new(node: Handle<Node>, old_position: Vec3, new_position: Vec3) -> Self {
        Self {
            node,
            old_position,
            new_position,
        }
    }

    fn swap(&mut self) -> Vec3 {
        let position = self.new_position;
        std::mem::swap(&mut self.new_position, &mut self.old_position);
        position
    }

    fn set_position(&self, graph: &mut Graph, position: Vec3) {
        graph[self.node]
            .local_transform_mut()
            .set_position(position);
    }
}

impl<'a> Command<'a> for MoveNodeCommand {
    type Context = SceneContext<'a>;

    fn execute(&mut self, context: &mut Self::Context) {
        let position = self.swap();
        self.set_position(context.graph, position);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let position = self.swap();
        self.set_position(context.graph, position);
    }
}

#[derive(Debug)]
pub struct ScaleNodeCommand {
    node: Handle<Node>,
    old_scale: Vec3,
    new_scale: Vec3,
}

impl ScaleNodeCommand {
    pub fn new(node: Handle<Node>, old_scale: Vec3, new_scale: Vec3) -> Self {
        Self {
            node,
            old_scale,
            new_scale,
        }
    }

    fn swap(&mut self) -> Vec3 {
        let position = self.new_scale;
        std::mem::swap(&mut self.new_scale, &mut self.old_scale);
        position
    }

    fn set_scale(&self, graph: &mut Graph, scale: Vec3) {
        graph[self.node].local_transform_mut().set_scale(scale);
    }
}

impl<'a> Command<'a> for ScaleNodeCommand {
    type Context = SceneContext<'a>;

    fn execute(&mut self, context: &mut Self::Context) {
        let scale = self.swap();
        self.set_scale(context.graph, scale);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let scale = self.swap();
        self.set_scale(context.graph, scale);
    }
}

#[derive(Debug)]
pub struct RotateNodeCommand {
    node: Handle<Node>,
    old_rotation: Quat,
    new_rotation: Quat,
}

impl RotateNodeCommand {
    pub fn new(node: Handle<Node>, old_rotation: Quat, new_rotation: Quat) -> Self {
        Self {
            node,
            old_rotation,
            new_rotation,
        }
    }

    fn swap(&mut self) -> Quat {
        let position = self.new_rotation;
        std::mem::swap(&mut self.new_rotation, &mut self.old_rotation);
        position
    }

    fn set_scale(&self, graph: &mut Graph, rotation: Quat) {
        graph[self.node]
            .local_transform_mut()
            .set_rotation(rotation);
    }
}

impl<'a> Command<'a> for RotateNodeCommand {
    type Context = SceneContext<'a>;

    fn execute(&mut self, context: &mut Self::Context) {
        let rotation = self.swap();
        self.set_scale(context.graph, rotation);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let rotation = self.swap();
        self.set_scale(context.graph, rotation);
    }
}

#[derive(Debug)]
pub struct LinkNodesCommand {
    child: Handle<Node>,
    parent: Handle<Node>,
}

impl LinkNodesCommand {
    pub fn new(child: Handle<Node>, parent: Handle<Node>) -> Self {
        Self { child, parent }
    }

    fn link(&mut self, graph: &mut Graph) {
        let old_parent = graph[self.child].parent();
        graph.link_nodes(self.child, self.parent);
        self.parent = old_parent;
    }
}

impl<'a> Command<'a> for LinkNodesCommand {
    type Context = SceneContext<'a>;

    fn execute(&mut self, context: &mut Self::Context) {
        self.link(context.graph);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.link(context.graph);
    }
}

#[derive(Debug)]
pub struct DeleteNodeCommand {
    handle: Handle<Node>,
    ticket: Option<Ticket<Node>>,
    node: Option<Node>,
}

impl DeleteNodeCommand {
    pub fn new(handle: Handle<Node>) -> Self {
        Self {
            handle,
            ticket: None,
            node: None,
        }
    }
}

impl<'a> Command<'a> for DeleteNodeCommand {
    type Context = SceneContext<'a>;

    fn execute(&mut self, context: &mut Self::Context) {
        let (ticket, node) = context.graph.take_reserve(self.handle);
        self.node = Some(node);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.handle = context
            .graph
            .put_back(self.ticket.take().unwrap(), self.node.take().unwrap());
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.graph.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct SetVisibleCommand {
    handle: Handle<Node>,
    value: bool,
}

impl SetVisibleCommand {
    pub fn new(handle: Handle<Node>, visible: bool) -> Self {
        Self {
            handle,
            value: visible,
        }
    }

    fn apply(&mut self, graph: &mut Graph) {
        let node = &mut graph[self.handle];
        let old = node.visibility();
        node.set_visibility(self.value);
        self.value = old;
    }
}

impl<'a> Command<'a> for SetVisibleCommand {
    type Context = SceneContext<'a>;

    fn execute(&mut self, context: &mut Self::Context) {
        self.apply(context.graph);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.apply(context.graph);
    }
}
