use crate::{
    command::Command, define_node_command, get_set_swap, physics::Physics,
    scene::commands::SceneContext,
};
use rg3d::{
    animation::Animation,
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::{Handle, Ticket},
    },
    engine::resource_manager::MaterialSearchOptions,
    scene::{
        base::PhysicsBinding,
        graph::{Graph, SubGraph},
        node::Node,
    },
};
use std::path::PathBuf;

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

    fn set_position(&self, graph: &mut Graph, physics: &mut Physics, position: Vector3<f32>) {
        graph[self.node]
            .local_transform_mut()
            .set_position(position);
        if let Some(&body) = physics.binder.value_of(&self.node) {
            physics.bodies[body].position = position;
        }
    }
}

impl<'a> Command<'a> for MoveNodeCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Move Node".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let position = self.swap();
        self.set_position(
            &mut context.scene.graph,
            &mut context.editor_scene.physics,
            position,
        );
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let position = self.swap();
        self.set_position(
            &mut context.scene.graph,
            &mut context.editor_scene.physics,
            position,
        );
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

impl<'a> Command<'a> for ScaleNodeCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Scale Node".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let scale = self.swap();
        self.set_scale(&mut context.scene.graph, scale);
    }

    fn revert(&mut self, context: &mut Self::Context) {
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

    fn set_rotation(
        &self,
        graph: &mut Graph,
        physics: &mut Physics,
        rotation: UnitQuaternion<f32>,
    ) {
        graph[self.node]
            .local_transform_mut()
            .set_rotation(rotation);
        if let Some(&body) = physics.binder.value_of(&self.node) {
            physics.bodies[body].rotation = rotation;
        }
    }
}

impl<'a> Command<'a> for RotateNodeCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Rotate Node".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let rotation = self.swap();
        self.set_rotation(
            &mut context.scene.graph,
            &mut context.editor_scene.physics,
            rotation,
        );
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let rotation = self.swap();
        self.set_rotation(
            &mut context.scene.graph,
            &mut context.editor_scene.physics,
            rotation,
        );
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

    fn name(&mut self, _context: &Self::Context) -> String {
        "Link Nodes".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.link(&mut context.scene.graph);
    }

    fn revert(&mut self, context: &mut Self::Context) {
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

impl DeleteNodeCommand {
    pub fn new(handle: Handle<Node>) -> Self {
        Self {
            handle,
            ticket: None,
            node: None,
            parent: Default::default(),
        }
    }
}

impl<'a> Command<'a> for DeleteNodeCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Delete Node".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.parent = context.scene.graph[self.handle].parent();
        let (ticket, node) = context.scene.graph.take_reserve(self.handle);
        self.node = Some(node);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.handle = context
            .scene
            .graph
            .put_back(self.ticket.take().unwrap(), self.node.take().unwrap());
        context.scene.graph.link_nodes(self.handle, self.parent);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.scene.graph.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct LoadModelCommand {
    path: PathBuf,
    model: Handle<Node>,
    animations: Vec<Handle<Animation>>,
    sub_graph: Option<SubGraph>,
    animations_container: Vec<(Ticket<Animation>, Animation)>,
    materials_search_options: MaterialSearchOptions,
}

impl LoadModelCommand {
    pub fn new(path: PathBuf, materials_search_options: MaterialSearchOptions) -> Self {
        Self {
            path,
            model: Default::default(),
            animations: Default::default(),
            sub_graph: None,
            animations_container: Default::default(),
            materials_search_options,
        }
    }
}

impl<'a> Command<'a> for LoadModelCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Load Model".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        if self.model.is_none() {
            // No model was loaded yet, do it.
            if let Ok(model) = rg3d::core::futures::executor::block_on(
                context
                    .resource_manager
                    .request_model(&self.path, self.materials_search_options.clone()),
            ) {
                let instance = model.instantiate(context.scene);
                self.model = instance.root;
                self.animations = instance.animations;

                // Enable instantiated animations.
                for &animation in self.animations.iter() {
                    context.scene.animations[animation].set_enabled(true);
                }
            }
        } else {
            // A model was loaded, but change was reverted and here we must put all nodes
            // back to graph.
            self.model = context
                .scene
                .graph
                .put_sub_graph_back(self.sub_graph.take().unwrap());
            for (ticket, animation) in self.animations_container.drain(..) {
                context.scene.animations.put_back(ticket, animation);
            }
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.sub_graph = Some(context.scene.graph.take_reserve_sub_graph(self.model));
        self.animations_container = self
            .animations
            .iter()
            .map(|&anim| context.scene.animations.take_reserve(anim))
            .collect();
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(sub_graph) = self.sub_graph.take() {
            context.scene.graph.forget_sub_graph(sub_graph)
        }
        for (ticket, _) in self.animations_container.drain(..) {
            context.scene.animations.forget_ticket(ticket);
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

impl<'a> Command<'a> for DeleteSubGraphCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Delete Sub Graph".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        self.parent = context.scene.graph[self.sub_graph_root].parent();
        self.sub_graph = Some(
            context
                .scene
                .graph
                .take_reserve_sub_graph(self.sub_graph_root),
        );
    }

    fn revert(&mut self, context: &mut Self::Context) {
        context
            .scene
            .graph
            .put_sub_graph_back(self.sub_graph.take().unwrap());
        context
            .scene
            .graph
            .link_nodes(self.sub_graph_root, self.parent);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
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
}

impl AddNodeCommand {
    pub fn new(node: Node) -> Self {
        Self {
            ticket: None,
            handle: Default::default(),
            cached_name: format!("Add Node {}", node.name()),
            node: Some(node),
        }
    }
}

impl<'a> Command<'a> for AddNodeCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        self.cached_name.clone()
    }

    fn execute(&mut self, context: &mut Self::Context) {
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
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let (ticket, node) = context.scene.graph.take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.node = Some(node);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.scene.graph.forget_ticket(ticket)
        }
    }
}

define_node_command!(SetNameCommand("Set Name", String) where fn swap(self, node) {
    get_set_swap!(self, node, name_owned, set_name);
});

define_node_command!(SetPhysicsBindingCommand("Set Physics Binding", PhysicsBinding) where fn swap(self, node) {
    get_set_swap!(self, node, physics_binding, set_physics_binding);
});

define_node_command!(SetTagCommand("Set Tag", String) where fn swap(self, node) {
    get_set_swap!(self, node, tag_owned, set_tag);
});

define_node_command!(SetVisibleCommand("Set Visible", bool) where fn swap(self, node) {
    get_set_swap!(self, node, visibility, set_visibility)
});
