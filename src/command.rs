use rg3d::{
    core::{
        pool::{Handle, Ticket},
        math::{vec3::Vec3, quat::Quat},
    },
    scene::{
        node::Node,
        graph::Graph,
    },
};

#[derive(Debug)]
pub enum Command {
    CommandGroup(Vec<Command>),
    AddNode(AddNodeCommand),
    DeleteNode(DeleteNodeCommand),
    ChangeSelection(ChangeSelectionCommand),
    MoveNode(MoveNodeCommand),
    ScaleNode(ScaleNodeCommand),
    RotateNode(RotateNodeCommand),
    LinkNodes(LinkNodesCommand),
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

    pub fn execute(&mut self, graph: &mut Graph) {
        match self.ticket.take() {
            None => {
                self.handle = graph.add_node(self.node.take().unwrap());
            }
            Some(ticket) => {
                graph.put_back(ticket, self.node.take().unwrap());
            }
        }
    }

    pub fn revert(&mut self, graph: &mut Graph) {
        let (ticket, node) = graph.take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.node = Some(node);
    }

    pub fn finalize(self, graph: &mut Graph) {
        graph.forget_ticket(self.ticket.unwrap())
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

    pub fn execute(&mut self) -> Handle<Node> {
        self.swap()
    }

    pub fn revert(&mut self) -> Handle<Node> {
        self.swap()
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
        graph[self.node].local_transform_mut().set_position(position);
    }

    pub fn execute(&mut self, graph: &mut Graph) {
        let position = self.swap();
        self.set_position(graph, position);
    }

    pub fn revert(&mut self, graph: &mut Graph) {
        let position = self.swap();
        self.set_position(graph, position);
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

    pub fn execute(&mut self, graph: &mut Graph) {
        let scale = self.swap();
        self.set_scale(graph, scale);
    }

    pub fn revert(&mut self, graph: &mut Graph) {
        let scale = self.swap();
        self.set_scale(graph, scale);
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
        graph[self.node].local_transform_mut().set_rotation(rotation);
    }

    pub fn execute(&mut self, graph: &mut Graph) {
        let rotation = self.swap();
        self.set_scale(graph, rotation);
    }

    pub fn revert(&mut self, graph: &mut Graph) {
        let rotation = self.swap();
        self.set_scale(graph, rotation);
    }
}

#[derive(Debug)]
pub struct LinkNodesCommand {
    child: Handle<Node>,
    parent: Handle<Node>
}

impl LinkNodesCommand {
    pub fn new(child: Handle<Node>, parent: Handle<Node>) -> Self{
        Self {
            child,
            parent
        }
    }

    fn link(&mut self, graph: &mut Graph) {
        let old_parent = graph[self.child].parent();
        graph.link_nodes(self.child, self.parent);
        self.parent = old_parent;
    }

    pub fn execute(&mut self, graph: &mut Graph) {
        self.link(graph);
    }

    pub fn revert(&mut self, graph: &mut Graph) {
        self.link(graph);
    }
}

#[derive(Debug)]
pub struct DeleteNodeCommand {
    handle: Handle<Node>,
    ticket: Option<Ticket<Node>>,
    node: Option<Node>
}

impl DeleteNodeCommand {
    pub fn new(handle: Handle<Node>) -> Self{
        Self {
            handle,
            ticket: None,
            node: None
        }
    }

    pub fn execute(&mut self, graph: &mut Graph) {
        let (ticket, node) = graph.take_reserve(self.handle);
        self.node = Some(node);
        self.ticket = Some(ticket);
    }

    pub fn revert(&mut self, graph: &mut Graph) {
        self.handle = graph.put_back(self.ticket.take().unwrap(), self.node.take().unwrap());
    }

    pub fn finalize(mut self, graph: &mut Graph) {
        if let Some(ticket) = self.ticket.take() {
            graph.forget_ticket(ticket)
        }
    }
}

pub struct CommandStack {
    commands: Vec<Command>,
    top: Option<usize>,
}

impl CommandStack {
    pub fn new() -> Self {
        Self {
            commands: Default::default(),
            top: None,
        }
    }

    #[must_use = "dropped commands must be finalized!"]
    pub fn add_command(&mut self, command: Command) -> Vec<Command> {
        let mut dropped_commands = Vec::default();
        if self.commands.is_empty() {
            self.top = Some(0);
        } else {
            // Advance top
            match self.top.as_mut() {
                None => self.top = Some(0),
                Some(top) => *top += 1,
            }
            // Drop everything after top.
            let top = self.top.unwrap_or(0);
            if top < self.commands.len() {
                dropped_commands = self.commands.drain(top..).collect();
            }
        }
        self.commands.push(command);
        dropped_commands
    }

    pub fn undo(&mut self) -> Option<&mut Command> {
        if self.commands.is_empty() {
            None
        } else {
            match self.top.as_mut() {
                None => None,
                Some(top) => {
                    let command = self.commands.get_mut(*top);
                    if *top == 0 {
                        self.top = None;
                    } else {
                        *top -= 1;
                    }
                    command
                }
            }
        }
    }

    pub fn redo(&mut self) -> Option<&mut Command> {
        if self.commands.is_empty() {
            None
        } else {
            match self.top.as_mut() {
                None => {
                    self.top = Some(1);
                    self.commands.first_mut()
                }
                Some(top) => {
                    let last = self.commands.len();
                    if *top < last {
                        let command = dbg!(self.commands.get_mut(*top));
                        *top += 1;
                        command
                    } else {
                        None
                    }
                }
            }
        }
    }
}