use crate::{
    camera::CameraController,
    command::Command,
    physics::{Collider, Joint, Physics, RigidBody},
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        color::Color,
        math::Matrix4Ext,
        pool::{Handle, Ticket},
    },
    engine::resource_manager::ResourceManager,
    resource::texture::Texture,
    scene::{
        graph::{Graph, SubGraph},
        mesh::Mesh,
        node::Node,
        physics::{ColliderShapeDesc, JointParamsDesc},
        Scene,
    },
    sound::pool::ErasedHandle,
};
use std::{path::PathBuf, sync::mpsc::Sender};

#[derive(Default)]
pub struct Clipboard {
    nodes: Vec<Node>,
}

impl Clipboard {
    pub fn clone_selection(&mut self, selection: &Selection, graph: &Graph) {
        self.nodes.clear();

        for &handle in selection.nodes() {
            self.nodes.push(graph.copy_single_node(handle));
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear()
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
}

pub struct EditorScene {
    pub path: Option<PathBuf>,
    pub scene: Handle<Scene>,
    // Handle to a root for all editor nodes.
    pub root: Handle<Node>,
    pub selection: Selection,
    pub clipboard: Clipboard,
    pub camera_controller: CameraController,
    pub physics: Physics,
}

#[derive(Debug)]
pub enum SceneCommand {
    CommandGroup(CommandGroup),
    AddNode(AddNodeCommand),
    DeleteNode(DeleteNodeCommand),
    DeleteSubGraph(DeleteSubGraphCommand),
    ChangeSelection(ChangeSelectionCommand),
    MoveNode(MoveNodeCommand),
    ScaleNode(ScaleNodeCommand),
    RotateNode(RotateNodeCommand),
    LinkNodes(LinkNodesCommand),
    SetVisible(SetVisibleCommand),
    SetName(SetNameCommand),
    AddJoint(AddJointCommand),
    DeleteJoint(DeleteJointCommand),
    SetJointConnectedBody(SetJointConnectedBodyCommand),
    SetBody(SetBodyCommand),
    SetBodyMass(SetBodyMassCommand),
    SetCollider(SetColliderCommand),
    SetColliderFriction(SetColliderFrictionCommand),
    SetColliderRestitution(SetColliderRestitutionCommand),
    SetColliderPosition(SetColliderPositionCommand),
    SetColliderRotation(SetColliderRotationCommand),
    SetCylinderHalfHeight(SetCylinderHalfHeightCommand),
    SetCylinderRadius(SetCylinderRadiusCommand),
    SetCapsuleRadius(SetCapsuleRadiusCommand),
    SetCapsuleBegin(SetCapsuleBeginCommand),
    SetCapsuleEnd(SetCapsuleEndCommand),
    SetConeHalfHeight(SetConeHalfHeightCommand),
    SetConeRadius(SetConeRadiusCommand),
    SetBallRadius(SetBallRadiusCommand),
    SetBallJointAnchor1(SetBallJointAnchor1Command),
    SetBallJointAnchor2(SetBallJointAnchor2Command),
    SetCuboidHalfExtents(SetCuboidHalfExtentsCommand),
    DeleteBody(DeleteBodyCommand),
    DeleteCollider(DeleteColliderCommand),
    LoadModel(LoadModelCommand),
    SetLightColor(SetLightColorCommand),
    SetLightScatter(SetLightScatterCommand),
    SetLightScatterEnabled(SetLightScatterEnabledCommand),
    SetLightCastShadows(SetLightCastShadowsCommand),
    SetPointLightRadius(SetPointLightRadiusCommand),
    SetSpotLightHotspot(SetSpotLightHotspotCommand),
    SetSpotLightFalloffAngleDelta(SetSpotLightFalloffAngleDeltaCommand),
    SetSpotLightDistance(SetSpotLightDistanceCommand),
    SetFov(SetFovCommand),
    SetZNear(SetZNearCommand),
    SetZFar(SetZFarCommand),
    SetParticleSystemAcceleration(SetParticleSystemAccelerationCommand),
    SetSpriteSize(SetSpriteSizeCommand),
    SetSpriteRotation(SetSpriteRotationCommand),
    SetSpriteColor(SetSpriteColorCommand),
    SetSpriteTexture(SetSpriteTextureCommand),
    SetMeshTexture(SetMeshTextureCommand),
    SetMeshCastShadows(SetMeshCastShadowsCommand),
}

pub struct SceneContext<'a> {
    pub editor_scene: &'a mut EditorScene,
    pub scene: &'a mut Scene,
    pub message_sender: Sender<Message>,
    pub resource_manager: ResourceManager,
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            SceneCommand::CommandGroup(v) => v.$func($($args),*),
            SceneCommand::AddNode(v) => v.$func($($args),*),
            SceneCommand::DeleteNode(v) => v.$func($($args),*),
            SceneCommand::ChangeSelection(v) => v.$func($($args),*),
            SceneCommand::MoveNode(v) => v.$func($($args),*),
            SceneCommand::ScaleNode(v) => v.$func($($args),*),
            SceneCommand::RotateNode(v) => v.$func($($args),*),
            SceneCommand::LinkNodes(v) => v.$func($($args),*),
            SceneCommand::SetVisible(v) => v.$func($($args),*),
            SceneCommand::SetName(v) => v.$func($($args),*),
            SceneCommand::SetBody(v) => v.$func($($args),*),
            SceneCommand::AddJoint(v) => v.$func($($args),*),
            SceneCommand::SetJointConnectedBody(v) => v.$func($($args),*),
            SceneCommand::DeleteJoint(v) => v.$func($($args),*),
            SceneCommand::DeleteSubGraph(v) => v.$func($($args),*),
            SceneCommand::SetBodyMass(v) => v.$func($($args),*),
            SceneCommand::SetCollider(v) => v.$func($($args),*),
            SceneCommand::SetColliderFriction(v) => v.$func($($args),*),
            SceneCommand::SetColliderRestitution(v) => v.$func($($args),*),
            SceneCommand::SetColliderPosition(v) => v.$func($($args),*),
            SceneCommand::SetColliderRotation(v) => v.$func($($args),*),
            SceneCommand::SetCylinderHalfHeight(v) => v.$func($($args),*),
            SceneCommand::SetCylinderRadius(v) => v.$func($($args),*),
            SceneCommand::SetCapsuleRadius(v) => v.$func($($args),*),
            SceneCommand::SetCapsuleBegin(v) => v.$func($($args),*),
            SceneCommand::SetCapsuleEnd(v) => v.$func($($args),*),
            SceneCommand::SetConeHalfHeight(v) => v.$func($($args),*),
            SceneCommand::SetConeRadius(v) => v.$func($($args),*),
            SceneCommand::SetBallRadius(v) => v.$func($($args),*),
            SceneCommand::SetBallJointAnchor1(v) => v.$func($($args),*),
            SceneCommand::SetBallJointAnchor2(v) => v.$func($($args),*),
            SceneCommand::SetCuboidHalfExtents(v) => v.$func($($args),*),
            SceneCommand::DeleteBody(v) => v.$func($($args),*),
            SceneCommand::DeleteCollider(v) => v.$func($($args),*),
            SceneCommand::LoadModel(v) => v.$func($($args),*),
            SceneCommand::SetLightColor(v) => v.$func($($args),*),
            SceneCommand::SetLightScatter(v) => v.$func($($args),*),
            SceneCommand::SetLightScatterEnabled(v) => v.$func($($args),*),
            SceneCommand::SetLightCastShadows(v) => v.$func($($args),*),
            SceneCommand::SetPointLightRadius(v) => v.$func($($args),*),
            SceneCommand::SetSpotLightHotspot(v) => v.$func($($args),*),
            SceneCommand::SetSpotLightFalloffAngleDelta(v) => v.$func($($args),*),
            SceneCommand::SetSpotLightDistance(v) => v.$func($($args),*),
            SceneCommand::SetFov(v) => v.$func($($args),*),
            SceneCommand::SetZNear(v) => v.$func($($args),*),
            SceneCommand::SetZFar(v) => v.$func($($args),*),
            SceneCommand::SetParticleSystemAcceleration(v) => v.$func($($args),*),
            SceneCommand::SetSpriteSize(v) => v.$func($($args),*),
            SceneCommand::SetSpriteRotation(v) => v.$func($($args),*),
            SceneCommand::SetSpriteColor(v) => v.$func($($args),*),
            SceneCommand::SetSpriteTexture(v) => v.$func($($args),*),
            SceneCommand::SetMeshTexture(v) => v.$func($($args),*),
            SceneCommand::SetMeshCastShadows(v) => v.$func($($args),*),
        }
    };
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

impl<'a> Command<'a> for CommandGroup {
    type Context = SceneContext<'a>;

    fn name(&mut self, context: &Self::Context) -> String {
        let mut name = String::from("Command group: ");
        for cmd in self.commands.iter_mut() {
            name.push_str(&cmd.name(context));
            name.push_str(", ");
        }
        name
    }

    fn execute(&mut self, context: &mut Self::Context) {
        for cmd in self.commands.iter_mut() {
            cmd.execute(context);
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        // revert must be done in reverse order.
        for cmd in self.commands.iter_mut().rev() {
            cmd.revert(context);
        }
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        for mut cmd in self.commands.drain(..) {
            cmd.finalize(context);
        }
    }
}

impl<'a> Command<'a> for SceneCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, context: &Self::Context) -> String {
        static_dispatch!(self, name, context)
    }

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

#[derive(Debug)]
pub struct AddJointCommand {
    ticket: Option<Ticket<Joint>>,
    handle: Handle<Joint>,
    joint: Option<Joint>,
}

impl AddJointCommand {
    pub fn new(node: Joint) -> Self {
        Self {
            ticket: None,
            handle: Default::default(),
            joint: Some(node),
        }
    }
}

impl<'a> Command<'a> for AddJointCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Add Joint".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        match self.ticket.take() {
            None => {
                self.handle = context
                    .editor_scene
                    .physics
                    .joints
                    .spawn(self.joint.take().unwrap());
            }
            Some(ticket) => {
                let handle = context
                    .editor_scene
                    .physics
                    .joints
                    .put_back(ticket, self.joint.take().unwrap());
                assert_eq!(handle, self.handle);
            }
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let (ticket, node) = context
            .editor_scene
            .physics
            .joints
            .take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.joint = Some(node);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.joints.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct DeleteJointCommand {
    handle: Handle<Joint>,
    ticket: Option<Ticket<Joint>>,
    node: Option<Joint>,
}

impl DeleteJointCommand {
    pub fn new(handle: Handle<Joint>) -> Self {
        Self {
            handle,
            ticket: None,
            node: None,
        }
    }
}

impl<'a> Command<'a> for DeleteJointCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Delete Joint".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let (ticket, node) = context
            .editor_scene
            .physics
            .joints
            .take_reserve(self.handle);
        self.node = Some(node);
        self.ticket = Some(ticket);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.handle = context
            .editor_scene
            .physics
            .joints
            .put_back(self.ticket.take().unwrap(), self.node.take().unwrap());
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.joints.forget_ticket(ticket)
        }
    }
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
}

impl<'a> Command<'a> for ChangeSelectionCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Change Selection".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let new_selection = self.swap();
        if new_selection != context.editor_scene.selection {
            context.editor_scene.selection = new_selection;
            context
                .message_sender
                .send(Message::SelectionChanged)
                .unwrap();
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
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
        if let Some(&body) = physics.binder.get(&self.node) {
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
        if let Some(&body) = physics.binder.get(&self.node) {
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
pub struct SetBodyCommand {
    node: Handle<Node>,
    ticket: Option<Ticket<RigidBody>>,
    handle: Handle<RigidBody>,
    body: Option<RigidBody>,
}

impl SetBodyCommand {
    pub fn new(node: Handle<Node>, body: RigidBody) -> Self {
        Self {
            node,
            ticket: None,
            handle: Default::default(),
            body: Some(body),
        }
    }
}

impl<'a> Command<'a> for SetBodyCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Set Node Body".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        match self.ticket.take() {
            None => {
                self.handle = context
                    .editor_scene
                    .physics
                    .bodies
                    .spawn(self.body.take().unwrap());
            }
            Some(ticket) => {
                context
                    .editor_scene
                    .physics
                    .bodies
                    .put_back(ticket, self.body.take().unwrap());
            }
        }
        context
            .editor_scene
            .physics
            .binder
            .insert(self.node, self.handle);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let (ticket, node) = context
            .editor_scene
            .physics
            .bodies
            .take_reserve(self.handle);
        self.ticket = Some(ticket);
        self.body = Some(node);
        context.editor_scene.physics.binder.remove(&self.node);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.bodies.forget_ticket(ticket);
            context.editor_scene.physics.binder.remove(&self.node);
        }
    }
}

#[derive(Debug)]
pub struct SetColliderCommand {
    body: Handle<RigidBody>,
    ticket: Option<Ticket<Collider>>,
    handle: Handle<Collider>,
    collider: Option<Collider>,
}

impl SetColliderCommand {
    pub fn new(body: Handle<RigidBody>, collider: Collider) -> Self {
        Self {
            body,
            ticket: None,
            handle: Default::default(),
            collider: Some(collider),
        }
    }
}

impl<'a> Command<'a> for SetColliderCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Set Collider".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        match self.ticket.take() {
            None => {
                self.handle = context
                    .editor_scene
                    .physics
                    .colliders
                    .spawn(self.collider.take().unwrap());
            }
            Some(ticket) => {
                context
                    .editor_scene
                    .physics
                    .colliders
                    .put_back(ticket, self.collider.take().unwrap());
            }
        }
        context.editor_scene.physics.colliders[self.handle].parent = self.body.into();
        context.editor_scene.physics.bodies[self.body]
            .colliders
            .push(self.handle.into());
    }

    fn revert(&mut self, context: &mut Self::Context) {
        let (ticket, mut collider) = context
            .editor_scene
            .physics
            .colliders
            .take_reserve(self.handle);
        collider.parent = Default::default();
        self.ticket = Some(ticket);
        self.collider = Some(collider);

        let body = &mut context.editor_scene.physics.bodies[self.body];
        body.colliders.remove(
            body.colliders
                .iter()
                .position(|&c| c == ErasedHandle::from(self.handle))
                .unwrap(),
        );
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.colliders.forget_ticket(ticket);
        }
    }
}

#[derive(Debug)]
pub struct LoadModelCommand {
    path: PathBuf,
    model: Handle<Node>,
    sub_graph: Option<SubGraph>,
}

impl LoadModelCommand {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            model: Default::default(),
            sub_graph: None,
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
            if let Ok(model) = rg3d::futures::executor::block_on(
                context.resource_manager.request_model(&self.path),
            ) {
                self.model = model.instantiate(context.scene).root;
            }
        } else {
            // A model was loaded, but change was reverted and here we must put all nodes
            // back to graph.
            self.model = context
                .scene
                .graph
                .put_sub_graph_back(self.sub_graph.take().unwrap());
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.sub_graph = Some(context.scene.graph.take_reserve_sub_graph(self.model));
    }

    fn finalize(&mut self, context: &mut Self::Context) {
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
pub struct DeleteBodyCommand {
    handle: Handle<RigidBody>,
    ticket: Option<Ticket<RigidBody>>,
    body: Option<RigidBody>,
    node: Handle<Node>,
}

impl DeleteBodyCommand {
    pub fn new(handle: Handle<RigidBody>) -> Self {
        Self {
            handle,
            ticket: None,
            body: None,
            node: Handle::NONE,
        }
    }
}

impl<'a> Command<'a> for DeleteBodyCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Delete Body".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let (ticket, node) = context
            .editor_scene
            .physics
            .bodies
            .take_reserve(self.handle);
        self.body = Some(node);
        self.ticket = Some(ticket);
        self.node = context.editor_scene.physics.unbind_by_body(self.handle);
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.handle = context
            .editor_scene
            .physics
            .bodies
            .put_back(self.ticket.take().unwrap(), self.body.take().unwrap());
        context
            .editor_scene
            .physics
            .binder
            .insert(self.node, self.handle);
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.bodies.forget_ticket(ticket)
        }
    }
}

#[derive(Debug)]
pub struct DeleteColliderCommand {
    handle: Handle<Collider>,
    ticket: Option<Ticket<Collider>>,
    collider: Option<Collider>,
    body: Handle<RigidBody>,
}

impl DeleteColliderCommand {
    pub fn new(handle: Handle<Collider>) -> Self {
        Self {
            handle,
            ticket: None,
            collider: None,
            body: Handle::NONE,
        }
    }
}

impl<'a> Command<'a> for DeleteColliderCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Delete Collider".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        let (ticket, collider) = context
            .editor_scene
            .physics
            .colliders
            .take_reserve(self.handle);
        self.body = collider.parent.into();
        self.collider = Some(collider);
        self.ticket = Some(ticket);

        let body = &mut context.editor_scene.physics.bodies[self.body];
        body.colliders.remove(
            body.colliders
                .iter()
                .position(|&c| c == ErasedHandle::from(self.handle))
                .unwrap(),
        );
    }

    fn revert(&mut self, context: &mut Self::Context) {
        self.handle = context
            .editor_scene
            .physics
            .colliders
            .put_back(self.ticket.take().unwrap(), self.collider.take().unwrap());

        let body = &mut context.editor_scene.physics.bodies[self.body];
        body.colliders.push(self.handle.into());
    }

    fn finalize(&mut self, context: &mut Self::Context) {
        if let Some(ticket) = self.ticket.take() {
            context.editor_scene.physics.colliders.forget_ticket(ticket)
        }
    }
}

macro_rules! define_simple_scene_command {
    ($name:ident, $human_readable_name:expr, $value_type:ty => $apply_method:expr ) => {
        #[derive(Debug)]
        pub struct $name {
            handle: Handle<Node>,
            value: $value_type,
        }

        impl $name {
            pub fn new(handle: Handle<Node>, value: $value_type) -> Self {
                Self { handle, value }
            }

            fn swap(&mut self, graph: &mut Graph) {
                $apply_method(self, graph)
            }
        }

        impl<'a> Command<'a> for $name {
            type Context = SceneContext<'a>;

            fn name(&mut self, _context: &Self::Context) -> String {
                $human_readable_name.to_owned()
            }

            fn execute(&mut self, context: &mut Self::Context) {
                self.swap(&mut context.scene.graph);
            }

            fn revert(&mut self, context: &mut Self::Context) {
                self.swap(&mut context.scene.graph);
            }
        }
    };
}

macro_rules! define_simple_physics_command {
    ($name:ident, $human_readable_name:expr, $handle_type:ty, $value_type:ty => $apply_method:expr ) => {
        #[derive(Debug)]
        pub struct $name {
            handle: $handle_type,
            value: $value_type,
        }

        impl $name {
            pub fn new(handle: $handle_type, value: $value_type) -> Self {
                Self { handle, value }
            }

            fn swap(&mut self, physics: &mut Physics) {
                $apply_method(self, physics)
            }
        }

        impl<'a> Command<'a> for $name {
            type Context = SceneContext<'a>;

            fn name(&mut self, _context: &Self::Context) -> String {
                $human_readable_name.to_owned()
            }

            fn execute(&mut self, context: &mut Self::Context) {
                self.swap(&mut context.editor_scene.physics);
            }

            fn revert(&mut self, context: &mut Self::Context) {
                self.swap(&mut context.editor_scene.physics);
            }
        }
    };
}

macro_rules! define_simple_body_command {
    ($name:ident, $human_readable_name:expr, $value_type:ty => $apply_method:expr ) => {
        define_simple_physics_command!($name, $human_readable_name, Handle<RigidBody>, $value_type => $apply_method);
    };
}

macro_rules! define_simple_collider_command {
    ($name:ident, $human_readable_name:expr, $value_type:ty => $apply_method:expr ) => {
        define_simple_physics_command!($name, $human_readable_name, Handle<Collider>, $value_type => $apply_method);
    };
}

macro_rules! define_simple_joint_command {
    ($name:ident, $human_readable_name:expr, $value_type:ty => $apply_method:expr ) => {
        define_simple_physics_command!($name, $human_readable_name, Handle<Joint>, $value_type => $apply_method);
    };
}

#[derive(Debug)]
enum TextureSet {
    Single(Texture),
    Multiple(Vec<Option<Texture>>),
}

#[derive(Debug)]
pub struct SetMeshTextureCommand {
    node: Handle<Node>,
    set: TextureSet,
}

impl SetMeshTextureCommand {
    pub fn new(node: Handle<Node>, texture: Texture) -> Self {
        Self {
            node,
            set: TextureSet::Single(texture),
        }
    }
}

impl<'a> Command<'a> for SetMeshTextureCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Set Texture".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
        if let TextureSet::Single(texture) = &self.set {
            let mesh: &mut Mesh = context.scene.graph[self.node].as_mesh_mut();
            let old_set = mesh
                .surfaces_mut()
                .iter()
                .map(|s| s.diffuse_texture())
                .collect();
            for surface in mesh.surfaces_mut() {
                surface.set_diffuse_texture(Some(texture.clone()));
            }
            self.set = TextureSet::Multiple(old_set);
        } else {
            unreachable!()
        }
    }

    fn revert(&mut self, context: &mut Self::Context) {
        if let TextureSet::Multiple(set) = &self.set {
            let mesh: &mut Mesh = context.scene.graph[self.node].as_mesh_mut();
            let new_value = mesh.surfaces_mut()[0].diffuse_texture().unwrap();
            assert_eq!(mesh.surfaces_mut().len(), set.len());
            for (surface, old_texture) in mesh.surfaces_mut().iter_mut().zip(set) {
                surface.set_diffuse_texture(old_texture.clone());
            }
            self.set = TextureSet::Single(new_value);
        } else {
            unreachable!()
        }
    }
}

define_simple_scene_command!(SetLightScatterCommand, "Set Light Scatter", Vector3<f32> => |this: &mut SetLightScatterCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_light_mut();
    let old = node.scatter();
    node.set_scatter(this.value);
    this.value = old;
});

define_simple_scene_command!(SetLightScatterEnabledCommand, "Set Light Scatter Enabled", bool => |this: &mut SetLightScatterEnabledCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_light_mut();
    let old = node.is_scatter_enabled();
    node.enable_scatter(this.value);
    this.value = old;
});

define_simple_scene_command!(SetLightCastShadowsCommand, "Set Light Cast Shadows", bool => |this: &mut SetLightCastShadowsCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_light_mut();
    let old = node.is_cast_shadows();
    node.set_cast_shadows(this.value);
    this.value = old;
});

define_simple_scene_command!(SetPointLightRadiusCommand, "Set Point Light Radius", f32 => |this: &mut SetPointLightRadiusCommand, graph: &mut Graph| {
    let point_light = graph[this.handle].as_light_mut().as_point_mut();
    let old = point_light.radius();
    point_light.set_radius(this.value);
    this.value = old;
});

define_simple_scene_command!(SetSpotLightHotspotCommand, "Set Spot Light Hotspot", f32 => |this: &mut SetSpotLightHotspotCommand, graph: &mut Graph| {
    let spot_light = graph[this.handle].as_light_mut().as_spot_mut();
    let old = spot_light.hotspot_cone_angle();
    spot_light.set_hotspot_cone_angle(this.value);
    this.value = old;
});

define_simple_scene_command!(SetSpotLightFalloffAngleDeltaCommand, "Set Spot Light Falloff Angle Delta", f32 => |this: &mut SetSpotLightFalloffAngleDeltaCommand, graph: &mut Graph| {
    let spot_light = graph[this.handle].as_light_mut().as_spot_mut();
    let old = spot_light.falloff_angle_delta();
    spot_light.set_falloff_angle_delta(this.value);
    this.value = old;
});

define_simple_scene_command!(SetSpotLightDistanceCommand, "Set Spot Light Distance", f32 => |this: &mut SetSpotLightDistanceCommand, graph: &mut Graph| {
    let spot_light = graph[this.handle].as_light_mut().as_spot_mut();
    let old = spot_light.distance();
    spot_light.set_distance(this.value);
    this.value = old;
});

define_simple_scene_command!(SetLightColorCommand, "Set Light Color", Color => |this: &mut SetLightColorCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_light_mut();
    let old = node.color();
    node.set_color(this.value);
    this.value = old;
});

define_simple_scene_command!(SetNameCommand, "Set Name", String => |this: &mut SetNameCommand, graph: &mut Graph| {
    let node = &mut graph[this.handle];
    let old = node.name().to_owned();
    node.set_name(&this.value);
    this.value = old;
});

define_simple_scene_command!(SetVisibleCommand, "Set Visible", bool => |this: &mut SetVisibleCommand, graph: &mut Graph| {
    let node = &mut graph[this.handle];
    let old = node.visibility();
    node.set_visibility(this.value);
    this.value = old;
});

define_simple_scene_command!(SetFovCommand, "Set Fov", f32 => |this: &mut SetFovCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_camera_mut();
    let old = node.fov();
    node.set_fov(this.value);
    this.value = old;
});

define_simple_scene_command!(SetZNearCommand, "Set Camera Z Near", f32 => |this: &mut SetZNearCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_camera_mut();
    let old = node.z_near();
    node.set_z_near(this.value);
    this.value = old;
});

define_simple_scene_command!(SetZFarCommand, "Set Camera Z Far", f32 => |this: &mut SetZFarCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_camera_mut();
    let old = node.z_far();
    node.set_z_far(this.value);
    this.value = old;
});

define_simple_scene_command!(SetParticleSystemAccelerationCommand, "Set Particle System Acceleration", Vector3<f32> => |this: &mut SetParticleSystemAccelerationCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_particle_system_mut();
    let old = node.acceleration();
    node.set_acceleration(this.value);
    this.value = old;
});

define_simple_scene_command!(SetSpriteSizeCommand, "Set Sprite Size", f32 => |this: &mut SetSpriteSizeCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_sprite_mut();
    let old = node.size();
    node.set_size(this.value);
    this.value = old;
});

define_simple_scene_command!(SetSpriteRotationCommand, "Set Sprite Rotation", f32 => |this: &mut SetSpriteRotationCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_sprite_mut();
    let old = node.rotation();
    node.set_rotation(this.value);
    this.value = old;
});

define_simple_scene_command!(SetSpriteColorCommand, "Set Sprite Color", Color => |this: &mut SetSpriteColorCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_sprite_mut();
    let old = node.color();
    node.set_color(this.value);
    this.value = old;
});

define_simple_scene_command!(SetSpriteTextureCommand, "Set Sprite Texture", Option<Texture> => |this: &mut SetSpriteTextureCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_sprite_mut();
    let old = node.texture();
    node.set_texture(this.value.clone());
    this.value = old;
});

define_simple_scene_command!(SetMeshCastShadowsCommand, "Set Mesh Cast Shadows", bool => |this: &mut SetMeshCastShadowsCommand, graph: &mut Graph| {
    let node = graph[this.handle].as_mesh_mut();
    let old = node.cast_shadows();
    node.set_cast_shadows(this.value.clone());
    this.value = old;
});

define_simple_body_command!(SetBodyMassCommand, "Set Body Mass", f32 => |this: &mut SetBodyMassCommand, physics: &mut Physics| {
    let body = &mut physics.bodies[this.handle];
    std::mem::swap(&mut body.mass, &mut this.value);
});

define_simple_collider_command!(SetColliderFrictionCommand, "Set Collider Friction", f32 => |this: &mut SetColliderFrictionCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    std::mem::swap(&mut collider.friction, &mut this.value);
});

define_simple_collider_command!(SetColliderRestitutionCommand, "Set Collider Restitution", f32 => |this: &mut SetColliderRestitutionCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    std::mem::swap(&mut collider.restitution, &mut this.value);
});

define_simple_collider_command!(SetColliderPositionCommand, "Set Collider Position", Vector3<f32> => |this: &mut SetColliderPositionCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    std::mem::swap(&mut collider.translation, &mut this.value);
});

define_simple_collider_command!(SetColliderRotationCommand, "Set Collider Rotation", UnitQuaternion<f32> => |this: &mut SetColliderRotationCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    std::mem::swap(&mut collider.rotation, &mut this.value);
});

define_simple_collider_command!(SetCylinderHalfHeightCommand, "Set Cylinder Half Height", f32 => |this: &mut SetCylinderHalfHeightCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    if let ColliderShapeDesc::Cylinder(cylinder) = &mut collider.shape {
        std::mem::swap(&mut cylinder.half_height, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_collider_command!(SetCylinderRadiusCommand, "Set Cylinder Radius", f32 => |this: &mut SetCylinderRadiusCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    if let ColliderShapeDesc::Cylinder(cylinder) = &mut collider.shape {
        std::mem::swap(&mut cylinder.radius, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_collider_command!(SetConeHalfHeightCommand, "Set Cone Half Height", f32 => |this: &mut SetConeHalfHeightCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    if let ColliderShapeDesc::Cone(cone) = &mut collider.shape {
        std::mem::swap(&mut cone.half_height, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_collider_command!(SetConeRadiusCommand, "Set Cone Radius", f32 => |this: &mut SetConeRadiusCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    if let ColliderShapeDesc::Cone(cone) = &mut collider.shape {
        std::mem::swap(&mut cone.radius, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_collider_command!(SetCuboidHalfExtentsCommand, "Set Cuboid Half Extents", Vector3<f32> => |this: &mut SetCuboidHalfExtentsCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    if let ColliderShapeDesc::Cuboid(cuboid) = &mut collider.shape {
        std::mem::swap(&mut cuboid.half_extents, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_collider_command!(SetCapsuleRadiusCommand, "Set Capsule Radius", f32 => |this: &mut SetCapsuleRadiusCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    if let ColliderShapeDesc::Capsule(capsule) = &mut collider.shape {
        std::mem::swap(&mut capsule.radius, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_collider_command!(SetCapsuleBeginCommand, "Set Capsule Begin", Vector3<f32> => |this: &mut SetCapsuleBeginCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    if let ColliderShapeDesc::Capsule(capsule) = &mut collider.shape {
        std::mem::swap(&mut capsule.begin, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_collider_command!(SetCapsuleEndCommand, "Set Capsule End", Vector3<f32> => |this: &mut SetCapsuleEndCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    if let ColliderShapeDesc::Capsule(capsule) = &mut collider.shape {
        std::mem::swap(&mut capsule.end, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_collider_command!(SetBallRadiusCommand, "Set Ball Radius", f32 => |this: &mut SetBallRadiusCommand, physics: &mut Physics| {
    let collider = &mut physics.colliders[this.handle];
    if let ColliderShapeDesc::Ball(ball) = &mut collider.shape {
        std::mem::swap(&mut ball.radius, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_joint_command!(SetBallJointAnchor1Command, "Set Ball Joint Anchor 1", Vector3<f32> => |this: &mut SetBallJointAnchor1Command, physics: &mut Physics| {
    let joint = &mut physics.joints[this.handle];
    if let JointParamsDesc::BallJoint(ball) = &mut joint.params {
        std::mem::swap(&mut ball.local_anchor1, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_joint_command!(SetBallJointAnchor2Command, "Set Ball Joint Anchor 2", Vector3<f32> => |this: &mut SetBallJointAnchor2Command, physics: &mut Physics| {
    let joint = &mut physics.joints[this.handle];
    if let JointParamsDesc::BallJoint(ball) = &mut joint.params {
        std::mem::swap(&mut ball.local_anchor2, &mut this.value);
    } else {
        unreachable!();
    }
});

define_simple_joint_command!(SetJointConnectedBodyCommand, "Set Joint Connected Body", ErasedHandle => |this: &mut SetJointConnectedBodyCommand, physics: &mut Physics| {
    let joint = &mut physics.joints[this.handle];
    std::mem::swap(&mut joint.body2, &mut this.value);
});

#[derive(Debug, Default, Clone, Eq)]
pub struct Selection {
    nodes: Vec<Handle<Node>>,
}

impl PartialEq for Selection {
    fn eq(&self, other: &Self) -> bool {
        if self.nodes.is_empty() && !other.nodes.is_empty() {
            false
        } else {
            // Selection is equal even when order of elements is different.
            // TODO: Find a way to do this faster.
            for &node in self.nodes.iter() {
                let mut found = false;
                for &other_node in other.nodes.iter() {
                    if other_node == node {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return false;
                }
            }
            true
        }
    }
}

impl Selection {
    pub fn from_list(nodes: Vec<Handle<Node>>) -> Self {
        Self {
            nodes: nodes.into_iter().filter(|h| h.is_some()).collect(),
        }
    }

    /// Creates new selection as single if node handle is not none, and empty if it is.
    pub fn single_or_empty(node: Handle<Node>) -> Self {
        if node.is_none() {
            Self {
                nodes: Default::default(),
            }
        } else {
            Self { nodes: vec![node] }
        }
    }

    /// Adds new selected node, or removes it if it is already in the list of selected nodes.
    pub fn insert_or_exclude(&mut self, handle: Handle<Node>) {
        if let Some(position) = self.nodes.iter().position(|&h| h == handle) {
            self.nodes.remove(position);
        } else {
            self.nodes.push(handle);
        }
    }

    pub fn contains(&self, handle: Handle<Node>) -> bool {
        self.nodes.iter().any(|&h| h == handle)
    }

    pub fn nodes(&self) -> &[Handle<Node>] {
        &self.nodes
    }

    pub fn is_multi_selection(&self) -> bool {
        self.nodes.len() > 1
    }

    pub fn is_single_selection(&self) -> bool {
        self.nodes.len() == 1
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn extend(&mut self, other: &Selection) {
        self.nodes.extend_from_slice(&other.nodes)
    }

    pub fn global_rotation_position(
        &self,
        graph: &Graph,
    ) -> Option<(UnitQuaternion<f32>, Vector3<f32>)> {
        if self.is_single_selection() {
            Some(graph.global_rotation_position_no_scale(self.nodes[0]))
        } else if self.is_empty() {
            None
        } else {
            let mut position = Vector3::default();
            let mut rotation = graph.global_rotation(self.nodes[0]);
            let t = 1.0 / self.nodes.len() as f32;
            for &handle in self.nodes.iter() {
                let global_transform = graph[handle].global_transform();
                position += global_transform.position();
                rotation = rotation.slerp(&graph.global_rotation(self.nodes[0]), t);
            }
            position = position.scale(t);
            Some((rotation, position))
        }
    }

    pub fn offset(&self, graph: &mut Graph, offset: Vector3<f32>) {
        for &handle in self.nodes.iter() {
            let global_scale = graph.global_scale(handle);
            let offset = Vector3::new(
                if global_scale.x.abs() > 0.0 {
                    offset.x / global_scale.x
                } else {
                    offset.x
                },
                if global_scale.y.abs() > 0.0 {
                    offset.y / global_scale.y
                } else {
                    offset.y
                },
                if global_scale.z.abs() > 0.0 {
                    offset.z / global_scale.z
                } else {
                    offset.z
                },
            );
            graph[handle].local_transform_mut().offset(offset);
        }
    }

    pub fn rotate(&self, graph: &mut Graph, rotation: UnitQuaternion<f32>) {
        for &handle in self.nodes.iter() {
            graph[handle].local_transform_mut().set_rotation(rotation);
        }
    }

    pub fn scale(&self, graph: &mut Graph, scale: Vector3<f32>) {
        for &handle in self.nodes.iter() {
            graph[handle].local_transform_mut().set_scale(scale);
        }
    }

    pub fn local_positions(&self, graph: &Graph) -> Vec<Vector3<f32>> {
        let mut positions = Vec::new();
        for &handle in self.nodes.iter() {
            positions.push(graph[handle].local_transform().position());
        }
        positions
    }

    pub fn local_rotations(&self, graph: &Graph) -> Vec<UnitQuaternion<f32>> {
        let mut rotations = Vec::new();
        for &handle in self.nodes.iter() {
            rotations.push(graph[handle].local_transform().rotation());
        }
        rotations
    }

    pub fn local_scales(&self, graph: &Graph) -> Vec<Vector3<f32>> {
        let mut scales = Vec::new();
        for &handle in self.nodes.iter() {
            scales.push(graph[handle].local_transform().scale());
        }
        scales
    }
}

fn is_descendant_of(handle: Handle<Node>, other: Handle<Node>, graph: &Graph) -> bool {
    for &child in graph[other].children() {
        if child == handle {
            return true;
        }

        let inner = is_descendant_of(handle, child, graph);
        if inner {
            return true;
        }
    }
    false
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
    let mut selection = editor_scene.selection.clone();
    if let Some(root_position) = selection.nodes.iter().position(|&n| n == graph.get_root()) {
        selection.nodes.remove(root_position);
    }

    // Change selection first.
    let mut command_group = CommandGroup::from(vec![SceneCommand::ChangeSelection(
        ChangeSelectionCommand::new(Default::default(), selection.clone()),
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

    let mut root_nodes = Vec::new();
    for &node in selection.nodes().iter() {
        let mut descendant = false;
        for &other_node in selection.nodes().iter() {
            if is_descendant_of(node, other_node, graph) {
                descendant = true;
                break;
            }
        }
        if !descendant {
            root_nodes.push(node);
        }
    }

    // Delete all associated physics entities in the whole hierarchy starting from root nodes
    // found above.
    let mut stack = root_nodes.clone();
    while let Some(node) = stack.pop() {
        if let Some(&body) = editor_scene.physics.binder.get(&node) {
            for &collider in editor_scene.physics.bodies[body].colliders.iter() {
                command_group.push(SceneCommand::DeleteCollider(DeleteColliderCommand::new(
                    collider.into(),
                )))
            }

            command_group.push(SceneCommand::DeleteBody(DeleteBodyCommand::new(body)));

            // Remove any associated joints.
            let joint = editor_scene.physics.find_joint(body);
            if joint.is_some() {
                command_group.push(SceneCommand::DeleteJoint(DeleteJointCommand::new(joint)));
            }

            // Also check if this node is attached to a joint as
            // "connected body".
            for (handle, joint) in editor_scene.physics.joints.pair_iter() {
                if joint.body2 == ErasedHandle::from(body) {
                    command_group.push(SceneCommand::SetJointConnectedBody(
                        SetJointConnectedBodyCommand::new(handle, ErasedHandle::none()),
                    ));
                }
            }
        }
        stack.extend_from_slice(graph[node].children());
    }

    for root_node in root_nodes {
        command_group.push(SceneCommand::DeleteSubGraph(DeleteSubGraphCommand::new(
            root_node,
        )));
    }

    SceneCommand::CommandGroup(command_group)
}
