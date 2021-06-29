use crate::scene::commands::sound::MoveSpatialSoundSourceCommand;
use crate::{
    command::Command,
    physics::{Collider, Joint, RigidBody},
    scene::{
        clipboard::DeepCloneResult,
        commands::{
            camera::{SetCameraPreviewCommand, SetFovCommand, SetZFarCommand, SetZNearCommand},
            graph::{
                AddNodeCommand, DeleteNodeCommand, DeleteSubGraphCommand, LinkNodesCommand,
                LoadModelCommand, MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand,
                SetNameCommand, SetPhysicsBindingCommand, SetTagCommand, SetVisibleCommand,
            },
            light::{
                SetLightCastShadowsCommand, SetLightColorCommand, SetLightScatterCommand,
                SetLightScatterEnabledCommand, SetPointLightRadiusCommand,
                SetSpotLightDistanceCommand, SetSpotLightFalloffAngleDeltaCommand,
                SetSpotLightHotspotCommand,
            },
            lod::{
                AddLodGroupLevelCommand, AddLodObjectCommand, ChangeLodRangeBeginCommand,
                ChangeLodRangeEndCommand, RemoveLodGroupLevelCommand, RemoveLodObjectCommand,
                SetLodGroupCommand,
            },
            mesh::{SetMeshCastShadowsCommand, SetMeshRenderPathCommand, SetMeshTextureCommand},
            navmesh::{
                AddNavmeshCommand, AddNavmeshEdgeCommand, AddNavmeshTriangleCommand,
                AddNavmeshVertexCommand, ConnectNavmeshEdgesCommand, DeleteNavmeshCommand,
                DeleteNavmeshVertexCommand, MoveNavmeshVertexCommand,
            },
            particle_system::{
                AddParticleSystemEmitterCommand, DeleteEmitterCommand,
                SetBoxEmitterHalfDepthCommand, SetBoxEmitterHalfHeightCommand,
                SetBoxEmitterHalfWidthCommand, SetCylinderEmitterHeightCommand,
                SetCylinderEmitterRadiusCommand, SetEmitterNumericParameterCommand,
                SetEmitterPositionCommand, SetParticleSystemAccelerationCommand,
                SetParticleSystemTextureCommand, SetSphereEmitterRadiusCommand,
            },
            physics::{
                AddJointCommand, DeleteBodyCommand, DeleteColliderCommand, DeleteJointCommand,
                SetBallJointAnchor1Command, SetBallJointAnchor2Command, SetBallRadiusCommand,
                SetBodyCommand, SetBodyMassCommand, SetCapsuleBeginCommand, SetCapsuleEndCommand,
                SetCapsuleRadiusCommand, SetColliderCollisionGroupsFilterCommand,
                SetColliderCollisionGroupsMembershipsCommand, SetColliderCommand,
                SetColliderFrictionCommand, SetColliderIsSensorCommand, SetColliderPositionCommand,
                SetColliderRestitutionCommand, SetColliderRotationCommand,
                SetConeHalfHeightCommand, SetConeRadiusCommand, SetCuboidHalfExtentsCommand,
                SetCylinderHalfHeightCommand, SetCylinderRadiusCommand,
                SetFixedJointAnchor1RotationCommand, SetFixedJointAnchor1TranslationCommand,
                SetFixedJointAnchor2RotationCommand, SetFixedJointAnchor2TranslationCommand,
                SetJointConnectedBodyCommand, SetPrismaticJointAnchor1Command,
                SetPrismaticJointAnchor2Command, SetPrismaticJointAxis1Command,
                SetPrismaticJointAxis2Command, SetRevoluteJointAnchor1Command,
                SetRevoluteJointAnchor2Command, SetRevoluteJointAxis1Command,
                SetRevoluteJointAxis2Command,
            },
            sound::{AddSoundSourceCommand, DeleteSoundSourceCommand},
            sound::{
                SetSoundSourceBufferCommand, SetSoundSourceGainCommand,
                SetSoundSourceLoopingCommand, SetSoundSourceNameCommand,
                SetSoundSourcePitchCommand, SetSoundSourcePlayOnceCommand,
                SetSpatialSoundSourceMaxDistanceCommand, SetSpatialSoundSourcePositionCommand,
                SetSpatialSoundSourceRadiusCommand, SetSpatialSoundSourceRolloffFactorCommand,
            },
            sprite::{
                SetSpriteColorCommand, SetSpriteRotationCommand, SetSpriteSizeCommand,
                SetSpriteTextureCommand,
            },
            terrain::{
                AddTerrainLayerCommand, DeleteTerrainLayerCommand, ModifyTerrainHeightCommand,
                ModifyTerrainLayerMaskCommand, SetTerrainLayerTextureCommand,
            },
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
use std::{collections::HashMap, sync::mpsc::Sender};

pub mod camera;
pub mod graph;
pub mod light;
pub mod lod;
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

#[derive(Debug)]
pub enum SceneCommand {
    // Generic commands.
    CommandGroup(CommandGroup),

    // Scene commands.
    Paste(PasteCommand),
    LoadModel(LoadModelCommand),

    // Graph commands.
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
    SetTag(SetTagCommand),

    // LOD commands.
    SetLodGroup(SetLodGroupCommand),
    AddLodGroupLevel(AddLodGroupLevelCommand),
    RemoveLodGroupLevel(RemoveLodGroupLevelCommand),
    AddLodObject(AddLodObjectCommand),
    RemoveLodObject(RemoveLodObjectCommand),
    ChangeLodRangeEnd(ChangeLodRangeEndCommand),
    ChangeLodRangeBegin(ChangeLodRangeBeginCommand),

    // Physics commands.
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
    SetColliderIsSensor(SetColliderIsSensorCommand),
    SetColliderCollisionGroupsMemberships(SetColliderCollisionGroupsMembershipsCommand),
    SetColliderCollisionGroupsFilter(SetColliderCollisionGroupsFilterCommand),
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
    SetFixedJointAnchor1Translation(SetFixedJointAnchor1TranslationCommand),
    SetFixedJointAnchor2Translation(SetFixedJointAnchor2TranslationCommand),
    SetFixedJointAnchor1Rotation(SetFixedJointAnchor1RotationCommand),
    SetFixedJointAnchor2Rotation(SetFixedJointAnchor2RotationCommand),
    SetRevoluteJointAnchor1(SetRevoluteJointAnchor1Command),
    SetRevoluteJointAxis1(SetRevoluteJointAxis1Command),
    SetRevoluteJointAnchor2(SetRevoluteJointAnchor2Command),
    SetRevoluteJointAxis2(SetRevoluteJointAxis2Command),
    SetPrismaticJointAnchor1(SetPrismaticJointAnchor1Command),
    SetPrismaticJointAxis1(SetPrismaticJointAxis1Command),
    SetPrismaticJointAnchor2(SetPrismaticJointAnchor2Command),
    SetPrismaticJointAxis2(SetPrismaticJointAxis2Command),
    SetCuboidHalfExtents(SetCuboidHalfExtentsCommand),
    DeleteBody(DeleteBodyCommand),
    DeleteCollider(DeleteColliderCommand),
    SetPhysicsBinding(SetPhysicsBindingCommand),

    // Light commands.
    SetLightColor(SetLightColorCommand),
    SetLightScatter(SetLightScatterCommand),
    SetLightScatterEnabled(SetLightScatterEnabledCommand),
    SetLightCastShadows(SetLightCastShadowsCommand),
    SetPointLightRadius(SetPointLightRadiusCommand),
    SetSpotLightHotspot(SetSpotLightHotspotCommand),
    SetSpotLightFalloffAngleDelta(SetSpotLightFalloffAngleDeltaCommand),
    SetSpotLightDistance(SetSpotLightDistanceCommand),

    // Camera commands.
    SetFov(SetFovCommand),
    SetZNear(SetZNearCommand),
    SetZFar(SetZFarCommand),
    SetCameraActive(SetCameraPreviewCommand),

    // Particle system commands.
    SetParticleSystemAcceleration(SetParticleSystemAccelerationCommand),
    AddParticleSystemEmitter(AddParticleSystemEmitterCommand),
    SetEmitterNumericParameter(SetEmitterNumericParameterCommand),
    SetSphereEmitterRadius(SetSphereEmitterRadiusCommand),
    SetCylinderEmitterRadius(SetCylinderEmitterRadiusCommand),
    SetCylinderEmitterHeight(SetCylinderEmitterHeightCommand),
    SetBoxEmitterHalfWidth(SetBoxEmitterHalfWidthCommand),
    SetBoxEmitterHalfHeight(SetBoxEmitterHalfHeightCommand),
    SetBoxEmitterHalfDepth(SetBoxEmitterHalfDepthCommand),
    SetEmitterPosition(SetEmitterPositionCommand),
    SetParticleSystemTexture(SetParticleSystemTextureCommand),
    DeleteEmitter(DeleteEmitterCommand),

    // Sprite commands.
    SetSpriteSize(SetSpriteSizeCommand),
    SetSpriteRotation(SetSpriteRotationCommand),
    SetSpriteColor(SetSpriteColorCommand),
    SetSpriteTexture(SetSpriteTextureCommand),

    // Mesh commands.
    SetMeshTexture(SetMeshTextureCommand),
    SetMeshCastShadows(SetMeshCastShadowsCommand),
    SetMeshRenderPath(SetMeshRenderPathCommand),

    // Navmesh commands.
    AddNavmesh(AddNavmeshCommand),
    DeleteNavmesh(DeleteNavmeshCommand),
    MoveNavmeshVertex(MoveNavmeshVertexCommand),
    AddNavmeshTriangle(AddNavmeshTriangleCommand),
    AddNavmeshVertex(AddNavmeshVertexCommand),
    AddNavmeshEdge(AddNavmeshEdgeCommand),
    DeleteNavmeshVertex(DeleteNavmeshVertexCommand),
    ConnectNavmeshEdges(ConnectNavmeshEdgesCommand),

    // Terrain commands.
    AddTerrainLayer(AddTerrainLayerCommand),
    DeleteTerrainLayer(DeleteTerrainLayerCommand),
    SetTerrainLayerTexture(SetTerrainLayerTextureCommand),
    ModifyTerrainHeight(ModifyTerrainHeightCommand),
    ModifyTerrainLayerMask(ModifyTerrainLayerMaskCommand),

    // Sound commands.
    AddSoundSource(AddSoundSourceCommand),
    DeleteSoundSource(DeleteSoundSourceCommand),
    MoveSpatialSoundSource(MoveSpatialSoundSourceCommand),
    SetSoundSourceGain(SetSoundSourceGainCommand),
    SetSoundSourceBuffer(SetSoundSourceBufferCommand),
    SetSoundSourceName(SetSoundSourceNameCommand),
    SetSoundSourcePitch(SetSoundSourcePitchCommand),
    SetSoundSourceLooping(SetSoundSourceLoopingCommand),
    SetSoundSourcePlayOnce(SetSoundSourcePlayOnceCommand),
    SetSpatialSoundSourcePosition(SetSpatialSoundSourcePositionCommand),
    SetSpatialSoundSourceRadius(SetSpatialSoundSourceRadiusCommand),
    SetSpatialSoundSourceRolloffFactor(SetSpatialSoundSourceRolloffFactorCommand),
    SetSpatialSoundSourceMaxDistance(SetSpatialSoundSourceMaxDistanceCommand),
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
            SceneCommand::Paste(v) => v.$func($($args),*),
            SceneCommand::AddNode(v) => v.$func($($args),*),
            SceneCommand::DeleteNode(v) => v.$func($($args),*),
            SceneCommand::ChangeSelection(v) => v.$func($($args),*),
            SceneCommand::MoveNode(v) => v.$func($($args),*),
            SceneCommand::ScaleNode(v) => v.$func($($args),*),
            SceneCommand::RotateNode(v) => v.$func($($args),*),
            SceneCommand::LinkNodes(v) => v.$func($($args),*),
            SceneCommand::SetVisible(v) => v.$func($($args),*),
            SceneCommand::SetName(v) => v.$func($($args),*),
            SceneCommand::SetLodGroup(v) => v.$func($($args),*),
            SceneCommand::AddLodGroupLevel(v) => v.$func($($args),*),
            SceneCommand::RemoveLodGroupLevel(v) => v.$func($($args),*),
            SceneCommand::AddLodObject(v) => v.$func($($args),*),
            SceneCommand::RemoveLodObject(v) => v.$func($($args),*),
            SceneCommand::ChangeLodRangeEnd(v) => v.$func($($args),*),
            SceneCommand::ChangeLodRangeBegin(v) => v.$func($($args),*),
            SceneCommand::SetTag(v) => v.$func($($args),*),
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
            SceneCommand::SetColliderIsSensor(v) => v.$func($($args),*),
            SceneCommand::SetColliderCollisionGroupsMemberships(v) => v.$func($($args),*),
            SceneCommand::SetColliderCollisionGroupsFilter(v) => v.$func($($args),*),
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
            SceneCommand::SetFixedJointAnchor1Translation(v) => v.$func($($args),*),
            SceneCommand::SetFixedJointAnchor2Translation(v) => v.$func($($args),*),
            SceneCommand::SetFixedJointAnchor1Rotation(v) => v.$func($($args),*),
            SceneCommand::SetFixedJointAnchor2Rotation(v) => v.$func($($args),*),
            SceneCommand::SetRevoluteJointAnchor1(v) => v.$func($($args),*),
            SceneCommand::SetRevoluteJointAxis1(v) => v.$func($($args),*),
            SceneCommand::SetRevoluteJointAnchor2(v) => v.$func($($args),*),
            SceneCommand::SetRevoluteJointAxis2(v) => v.$func($($args),*),
            SceneCommand::SetPrismaticJointAnchor1(v) => v.$func($($args),*),
            SceneCommand::SetPrismaticJointAxis1(v) => v.$func($($args),*),
            SceneCommand::SetPrismaticJointAnchor2(v) => v.$func($($args),*),
            SceneCommand::SetPrismaticJointAxis2(v) => v.$func($($args),*),
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
            SceneCommand::SetCameraActive(v) => v.$func($($args),*),
            SceneCommand::SetParticleSystemAcceleration(v) => v.$func($($args),*),
            SceneCommand::AddParticleSystemEmitter(v) => v.$func($($args),*),
            SceneCommand::SetEmitterNumericParameter(v) => v.$func($($args),*),
            SceneCommand::SetSphereEmitterRadius(v) => v.$func($($args),*),
            SceneCommand::SetEmitterPosition(v) => v.$func($($args),*),
            SceneCommand::SetParticleSystemTexture(v) => v.$func($($args),*),
            SceneCommand::SetCylinderEmitterRadius(v) => v.$func($($args),*),
            SceneCommand::SetCylinderEmitterHeight(v) => v.$func($($args),*),
            SceneCommand::SetBoxEmitterHalfWidth(v) => v.$func($($args),*),
            SceneCommand::SetBoxEmitterHalfHeight(v) => v.$func($($args),*),
            SceneCommand::SetBoxEmitterHalfDepth(v) => v.$func($($args),*),
            SceneCommand::DeleteEmitter(v) => v.$func($($args),*),
            SceneCommand::SetSpriteSize(v) => v.$func($($args),*),
            SceneCommand::SetSpriteRotation(v) => v.$func($($args),*),
            SceneCommand::SetSpriteColor(v) => v.$func($($args),*),
            SceneCommand::SetSpriteTexture(v) => v.$func($($args),*),
            SceneCommand::SetMeshTexture(v) => v.$func($($args),*),
            SceneCommand::SetMeshCastShadows(v) => v.$func($($args),*),
            SceneCommand::SetMeshRenderPath(v) => v.$func($($args),*),
            SceneCommand::AddNavmesh(v) => v.$func($($args),*),
            SceneCommand::DeleteNavmesh(v) => v.$func($($args),*),
            SceneCommand::MoveNavmeshVertex(v) => v.$func($($args),*),
            SceneCommand::AddNavmeshVertex(v) => v.$func($($args),*),
            SceneCommand::AddNavmeshTriangle(v) => v.$func($($args),*),
            SceneCommand::AddNavmeshEdge(v) => v.$func($($args),*),
            SceneCommand::DeleteNavmeshVertex(v) => v.$func($($args),*),
            SceneCommand::ConnectNavmeshEdges(v) => v.$func($($args),*),
            SceneCommand::SetPhysicsBinding(v) => v.$func($($args),*),
            SceneCommand::AddTerrainLayer(v) => v.$func($($args),*),
            SceneCommand::DeleteTerrainLayer(v) => v.$func($($args),*),
            SceneCommand::SetTerrainLayerTexture(v) => v.$func($($args),*),
            SceneCommand::ModifyTerrainHeight(v) => v.$func($($args),*),
            SceneCommand::ModifyTerrainLayerMask(v) => v.$func($($args),*),
            SceneCommand::AddSoundSource(v) => v.$func($($args),*),
            SceneCommand::DeleteSoundSource(v) => v.$func($($args),*),
            SceneCommand::MoveSpatialSoundSource(v) => v.$func($($args),*),
            SceneCommand::SetSoundSourceGain(v) => v.$func($($args),*),
            SceneCommand::SetSoundSourceBuffer(v) => v.$func($($args),*),
            SceneCommand::SetSoundSourceName(v) => v.$func($($args),*),
            SceneCommand::SetSoundSourcePitch(v) => v.$func($($args),*),
            SceneCommand::SetSoundSourceLooping(v) => v.$func($($args),*),
            SceneCommand::SetSoundSourcePlayOnce(v) => v.$func($($args),*),
            SceneCommand::SetSpatialSoundSourcePosition(v) => v.$func($($args),*),
            SceneCommand::SetSpatialSoundSourceRadius(v) => v.$func($($args),*),
            SceneCommand::SetSpatialSoundSourceRolloffFactor(v) => v.$func($($args),*),
            SceneCommand::SetSpatialSoundSourceMaxDistance(v) => v.$func($($args),*),
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
    let mut command_group = CommandGroup::from(vec![SceneCommand::ChangeSelection(
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

impl<'a> Command<'a> for ChangeSelectionCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        self.cached_name.clone()
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

impl<'a> Command<'a> for PasteCommand {
    type Context = SceneContext<'a>;

    fn name(&mut self, _context: &Self::Context) -> String {
        "Paste".to_owned()
    }

    fn execute(&mut self, context: &mut Self::Context) {
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

    fn revert(&mut self, context: &mut Self::Context) {
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

    fn finalize(&mut self, context: &mut Self::Context) {
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
