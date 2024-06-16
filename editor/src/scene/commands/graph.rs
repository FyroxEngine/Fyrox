use crate::{
    command::{CommandContext, CommandTrait},
    fyrox::{
        core::{
            algebra::{UnitQuaternion, Vector3},
            pool::{Handle, Ticket},
        },
        graph::{BaseSceneGraph, LinkScheme, SceneGraphNode},
        scene::{
            base::Base,
            graph::{Graph, SubGraph},
            node::Node,
            transform::Transform,
        },
    },
    scene::{commands::GameSceneContext, Selection},
    world::graph::selection::GraphSelection,
    Message,
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

impl CommandTrait for MoveNodeCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Move Node".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let position = self.swap();
        self.set_position(&mut context.scene.graph, position);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let position = self.swap();
        self.set_position(&mut context.scene.graph, position);
    }
}

#[derive(Debug)]
pub struct SetNodeTransformCommand {
    node: Handle<Node>,
    old_transform: Transform,
    new_transform: Transform,
}

impl SetNodeTransformCommand {
    pub fn new(node: Handle<Node>, old_transform: Transform, new_transform: Transform) -> Self {
        Self {
            node,
            old_transform,
            new_transform,
        }
    }

    fn swap(&mut self) -> Transform {
        let transform = self.new_transform.clone();
        std::mem::swap(&mut self.new_transform, &mut self.old_transform);
        transform
    }

    fn set_transform(&self, graph: &mut Graph, transform: Transform) {
        *graph[self.node].local_transform_mut() = transform;
    }
}

impl CommandTrait for SetNodeTransformCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Node Transform".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let transform = self.swap();
        self.set_transform(&mut context.scene.graph, transform);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let transform = self.swap();
        self.set_transform(&mut context.scene.graph, transform);
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

impl CommandTrait for ScaleNodeCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Scale Node".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let scale = self.swap();
        self.set_scale(&mut context.scene.graph, scale);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

impl CommandTrait for RotateNodeCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Rotate Node".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let rotation = self.swap();
        self.set_rotation(&mut context.scene.graph, rotation);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

impl CommandTrait for LinkNodesCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Link Nodes".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.link(&mut context.scene.graph);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.link(&mut context.scene.graph);
    }
}

#[derive(Debug)]
pub struct SetGraphNodeChildPosition {
    pub node: Handle<Node>,
    pub child: Handle<Node>,
    pub position: usize,
}

impl SetGraphNodeChildPosition {
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        let prev_pos = context.scene.graph[self.node]
            .set_child_position(self.child, self.position)
            .unwrap();
        self.position = prev_pos;
    }
}

impl CommandTrait for SetGraphNodeChildPosition {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Child Position".to_string()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct DeleteNodeCommand {
    handle: Handle<Node>,
    ticket: Option<Ticket<Node>>,
    node: Option<Node>,
    parent: Handle<Node>,
}

impl CommandTrait for DeleteNodeCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Delete Node".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.parent = context.scene.graph[self.handle].parent();
        let (ticket, node) = context.scene.graph.take_reserve(self.handle);
        self.node = Some(node);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.handle = context
            .scene
            .graph
            .put_back(self.ticket.take().unwrap(), self.node.take().unwrap());
        context.scene.graph.link_nodes(self.handle, self.parent);
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

impl CommandTrait for AddModelCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Load Model".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        // A model was loaded, but change was reverted and here we must put all nodes
        // back to graph.
        self.model = context
            .scene
            .graph
            .put_sub_graph_back(self.sub_graph.take().unwrap());
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.sub_graph = Some(context.scene.graph.take_reserve_sub_graph(self.model));
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

impl CommandTrait for DeleteSubGraphCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Delete Sub Graph".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.parent = context.scene.graph[self.sub_graph_root].parent();
        self.sub_graph = Some(
            context
                .scene
                .graph
                .take_reserve_sub_graph(self.sub_graph_root),
        );
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        context
            .scene
            .graph
            .put_sub_graph_back(self.sub_graph.take().unwrap());
        context
            .scene
            .graph
            .link_nodes(self.sub_graph_root, self.parent);
    }

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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
            prev_selection: Selection::new_empty(),
        }
    }
}

impl CommandTrait for AddNodeCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        self.cached_name.clone()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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
                Selection::new(GraphSelection::single_or_empty(self.handle)),
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

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

    fn finalize(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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
    fn swap(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
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

impl CommandTrait for ReplaceNodeCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Replace Node".to_owned()
    }

    fn execute(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        self.swap(context);
    }
}

#[derive(Debug)]
pub struct SetGraphRootCommand {
    pub root: Handle<Node>,
    pub link_scheme: LinkScheme<Node>,
}

impl CommandTrait for SetGraphRootCommand {
    fn name(&mut self, _context: &dyn CommandContext) -> String {
        "Set Graph Root".to_string()
    }

    #[allow(clippy::unnecessary_to_owned)] // false positive
    fn execute(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        self.link_scheme = context
            .scene
            .graph
            .change_hierarchy_root(*context.scene_content_root, self.root);
        self.root = std::mem::replace(context.scene_content_root, self.root);
    }

    fn revert(&mut self, context: &mut dyn CommandContext) {
        let context = context.get_mut::<GameSceneContext>();
        context
            .scene
            .graph
            .apply_link_scheme(std::mem::take(&mut self.link_scheme));
        self.root = std::mem::replace(context.scene_content_root, self.root);
    }
}
