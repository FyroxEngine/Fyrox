use crate::{
    command::GameSceneCommandTrait, scene::commands::GameSceneContext, scene::Selection,
    world::graph::selection::GraphSelection, Message,
};
use fyrox::graph::SceneGraph;
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::{Handle, Ticket},
    },
    scene::{
        base::Base,
        graph::{Graph, SubGraph},
        node::Node,
    },
};

#[derive(Debug)]
pub struct MoveNodeCommand {
    node: Handle<Node>,
    old_position: Vector3<f32>,
    new_position: Vector3<f32>,
}

impl MoveNodeCommand {
    pub fn new(node: Handle<Node>, old_position: Vector3<f32>, new_position: Vector3<f32>) -> Self {
        Self {
            node,
            old_position,
            new_position,
        }
    }

    fn swap(&mut self) -> Vector3<f32> {
        let position = self.new_position;
        std::mem::swap(&mut self.new_position, &mut self.old_position);
        position
    }

    fn set_position(&self, graph: &mut Graph, position: Vector3<f32>) {
        graph[self.node]
            .local_transform_mut()
            .set_position(position);
    }
}

impl GameSceneCommandTrait for MoveNodeCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Move Node".to_owned()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        let position = self.swap();
        self.set_position(&mut context.scene.graph, position);
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        let position = self.swap();
        self.set_position(&mut context.scene.graph, position);
    }
}

#[derive(Debug)]
pub struct ScaleNodeCommand {
    node: Handle<Node>,
    old_scale: Vector3<f32>,
    new_scale: Vector3<f32>,
}

impl ScaleNodeCommand {
    pub fn new(node: Handle<Node>, old_scale: Vector3<f32>, new_scale: Vector3<f32>) -> Self {
        Self {
            node,
            old_scale,
            new_scale,
        }
    }

    fn swap(&mut self) -> Vector3<f32> {
        let position = self.new_scale;
        std::mem::swap(&mut self.new_scale, &mut self.old_scale);
        position
    }

    fn set_scale(&self, graph: &mut Graph, scale: Vector3<f32>) {
        graph[self.node].local_transform_mut().set_scale(scale);
    }
}

impl GameSceneCommandTrait for ScaleNodeCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Scale Node".to_owned()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        let scale = self.swap();
        self.set_scale(&mut context.scene.graph, scale);
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        let scale = self.swap();
        self.set_scale(&mut context.scene.graph, scale);
    }
}

#[derive(Debug)]
pub struct RotateNodeCommand {
    node: Handle<Node>,
    old_rotation: UnitQuaternion<f32>,
    new_rotation: UnitQuaternion<f32>,
}

impl RotateNodeCommand {
    pub fn new(
        node: Handle<Node>,
        old_rotation: UnitQuaternion<f32>,
        new_rotation: UnitQuaternion<f32>,
    ) -> Self {
        Self {
            node,
            old_rotation,
            new_rotation,
        }
    }

    fn swap(&mut self) -> UnitQuaternion<f32> {
        let position = self.new_rotation;
        std::mem::swap(&mut self.new_rotation, &mut self.old_rotation);
        position
    }

    fn set_rotation(&self, graph: &mut Graph, rotation: UnitQuaternion<f32>) {
        graph[self.node]
            .local_transform_mut()
            .set_rotation(rotation);
    }
}

impl GameSceneCommandTrait for RotateNodeCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Rotate Node".to_owned()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        let rotation = self.swap();
        self.set_rotation(&mut context.scene.graph, rotation);
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        let rotation = self.swap();
        self.set_rotation(&mut context.scene.graph, rotation);
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

impl GameSceneCommandTrait for LinkNodesCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Link Nodes".to_owned()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        self.link(&mut context.scene.graph);
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        self.link(&mut context.scene.graph);
    }
}

#[derive(Debug)]
pub struct DeleteNodeCommand {
    handle: Handle<Node>,
    ticket: Option<Ticket<Node>>,
    node: Option<Node>,
    parent: Handle<Node>,
}

impl GameSceneCommandTrait for DeleteNodeCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Delete Node".to_owned()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        self.parent = context.scene.graph[self.handle].parent();
        let (ticket, node) = context.scene.graph.take_reserve(self.handle);
        self.node = Some(node);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        self.handle = context
            .scene
            .graph
            .put_back(self.ticket.take().unwrap(), self.node.take().unwrap());
        context.scene.graph.link_nodes(self.handle, self.parent);
    }

    fn finalize(&mut self, context: &mut GameSceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context
                .scene
                .graph
                .forget_ticket(ticket, self.node.take().unwrap());
        }
    }
}

#[derive(Debug)]
pub struct AddModelCommand {
    model: Handle<Node>,
    sub_graph: Option<SubGraph>,
}

impl AddModelCommand {
    pub fn new(sub_graph: SubGraph) -> Self {
        Self {
            model: Default::default(),
            sub_graph: Some(sub_graph),
        }
    }
}

impl GameSceneCommandTrait for AddModelCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Load Model".to_owned()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        // A model was loaded, but change was reverted and here we must put all nodes
        // back to graph.
        self.model = context
            .scene
            .graph
            .put_sub_graph_back(self.sub_graph.take().unwrap());
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        self.sub_graph = Some(context.scene.graph.take_reserve_sub_graph(self.model));
    }

    fn finalize(&mut self, context: &mut GameSceneContext) {
        if let Some(sub_graph) = self.sub_graph.take() {
            context.scene.graph.forget_sub_graph(sub_graph)
        }
    }
}

#[derive(Debug)]
pub struct DeleteSubGraphCommand {
    sub_graph_root: Handle<Node>,
    sub_graph: Option<SubGraph>,
    parent: Handle<Node>,
}

impl DeleteSubGraphCommand {
    pub fn new(sub_graph_root: Handle<Node>) -> Self {
        Self {
            sub_graph_root,
            sub_graph: None,
            parent: Handle::NONE,
        }
    }
}

impl GameSceneCommandTrait for DeleteSubGraphCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Delete Sub Graph".to_owned()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        self.parent = context.scene.graph[self.sub_graph_root].parent();
        self.sub_graph = Some(
            context
                .scene
                .graph
                .take_reserve_sub_graph(self.sub_graph_root),
        );
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        context
            .scene
            .graph
            .put_sub_graph_back(self.sub_graph.take().unwrap());
        context
            .scene
            .graph
            .link_nodes(self.sub_graph_root, self.parent);
    }

    fn finalize(&mut self, context: &mut GameSceneContext) {
        if let Some(sub_graph) = self.sub_graph.take() {
            context.scene.graph.forget_sub_graph(sub_graph)
        }
    }
}

#[derive(Debug)]
pub struct AddNodeCommand {
    ticket: Option<Ticket<Node>>,
    handle: Handle<Node>,
    node: Option<Node>,
    cached_name: String,
    parent: Handle<Node>,
    select_added: bool,
    prev_selection: Selection,
}

impl AddNodeCommand {
    pub fn new(node: Node, parent: Handle<Node>, select_added: bool) -> Self {
        Self {
            ticket: None,
            handle: Default::default(),
            cached_name: format!("Add Node {}", node.name()),
            node: Some(node),
            parent,
            select_added,
            prev_selection: Selection::None,
        }
    }
}

impl GameSceneCommandTrait for AddNodeCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        self.cached_name.clone()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        match self.ticket.take() {
            None => {
                self.handle = context.scene.graph.add_node(self.node.take().unwrap());
            }
            Some(ticket) => {
                let handle = context
                    .scene
                    .graph
                    .put_back(ticket, self.node.take().unwrap());
                assert_eq!(handle, self.handle);
            }
        }

        if self.select_added {
            self.prev_selection = std::mem::replace(
                context.selection,
                Selection::Graph(GraphSelection::single_or_empty(self.handle)),
            );
            context.message_sender.send(Message::SelectionChanged {
                old_selection: self.prev_selection.clone(),
            });
        }

        context.scene.graph.link_nodes(
            self.handle,
            if self.parent.is_none() {
                *context.scene_content_root
            } else {
                self.parent
            },
        )
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        // No need to unlink node from its parent because .take_reserve() does that for us.
        let (ticket, node) = context.scene.graph.take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.node = Some(node);

        if self.select_added {
            std::mem::swap(context.selection, &mut self.prev_selection);
            context.message_sender.send(Message::SelectionChanged {
                old_selection: self.prev_selection.clone(),
            });
        }
    }

    fn finalize(&mut self, context: &mut GameSceneContext) {
        if let Some(ticket) = self.ticket.take() {
            context
                .scene
                .graph
                .forget_ticket(ticket, self.node.take().unwrap());
        }
    }
}

#[derive(Debug)]
pub struct ReplaceNodeCommand {
    pub handle: Handle<Node>,
    pub node: Node,
}

impl ReplaceNodeCommand {
    fn swap(&mut self, context: &mut GameSceneContext) {
        let existing = &mut context.scene.graph[self.handle];

        // Swap `Base` part, this is needed because base part contains hierarchy info.
        // This way base part will be moved to replacement node.
        let existing_base: &mut Base = existing;
        let replacement_base: &mut Base = &mut self.node;

        std::mem::swap(existing_base, replacement_base);

        // Now swap them completely.
        std::mem::swap(existing, &mut self.node);
    }
}

impl GameSceneCommandTrait for ReplaceNodeCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Replace Node".to_owned()
    }

    fn execute(&mut self, context: &mut GameSceneContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        self.swap(context);
    }
}

#[derive(Debug)]
pub struct SetGraphRootCommand {
    pub root: Handle<Node>,
    pub revert_list: Vec<(Handle<Node>, Handle<Node>)>,
}

impl GameSceneCommandTrait for SetGraphRootCommand {
    fn name(&mut self, _context: &GameSceneContext) -> String {
        "Set Graph Root".to_string()
    }

    #[allow(clippy::unnecessary_to_owned)] // false positive
    fn execute(&mut self, context: &mut GameSceneContext) {
        let graph = &mut context.scene.graph;
        let prev_root = *context.scene_content_root;
        self.revert_list
            .push((self.root, graph[self.root].parent()));
        graph.link_nodes(self.root, graph.get_root());
        for prev_root_child in graph[prev_root].children().to_vec() {
            graph.link_nodes(prev_root_child, self.root);
            self.revert_list.push((prev_root_child, prev_root));
        }
        graph.link_nodes(prev_root, self.root);
        self.revert_list.push((prev_root, graph.get_root()));

        self.root = std::mem::replace(context.scene_content_root, self.root);
    }

    fn revert(&mut self, context: &mut GameSceneContext) {
        for (child, parent) in self.revert_list.drain(..) {
            context.scene.graph.link_nodes(child, parent);
        }
        self.root = std::mem::replace(context.scene_content_root, self.root);
    }
}
