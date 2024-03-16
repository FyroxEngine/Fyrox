//! Joint is used to restrict motion of two rigid bodies.

use crate::{
    core::{
        algebra::{Isometry2, Matrix4, UnitComplex, Vector2},
        log::Log,
        math::{aabb::AxisAlignedBoundingBox, m4x4_approx_eq},
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    scene::{
        base::{Base, BaseBuilder},
        dim2::rigidbody::RigidBody,
        graph::Graph,
        node::{Node, NodeTrait, SyncContext},
        Scene,
    },
};
use fyrox_core::uuid_provider;
use fyrox_graph::BaseSceneGraph;
use rapier2d::dynamics::ImpulseJointHandle;
use std::{
    cell::{Cell, RefCell},
    ops::{Deref, DerefMut, Range},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Ball joint locks any translational moves between two objects on the axis between objects, but
/// allows rigid bodies to perform relative rotations. The real world example is a human shoulder,
/// pendulum, etc.
#[derive(Clone, Debug, Visit, PartialEq, Reflect)]
pub struct BallJoint {
    /// Whether angular limits are enabled or not. Default is `false`
    #[reflect(description = "Whether angular limits are enabled or not.")]
    #[visit(optional)] // Backward compatibility
    pub limits_enabled: bool,

    /// Allowed angles range for the joint (in radians).
    #[reflect(description = "Allowed angles range for the joint (in radians).")]
    #[visit(optional)] // Backward compatibility
    pub limits_angles: Range<f32>,
}

impl Default for BallJoint {
    fn default() -> Self {
        Self {
            limits_enabled: false,
            limits_angles: -std::f32::consts::PI..std::f32::consts::PI,
        }
    }
}

/// A fixed joint ensures that two rigid bodies does not move relative to each other. There is no
/// straightforward real-world example, but it can be thought as two bodies were "welded" together.
#[derive(Clone, Debug, Default, Visit, PartialEq, Reflect, Eq)]
pub struct FixedJoint;

/// Prismatic joint prevents any relative movement between two rigid-bodies, except for relative
/// translations along one axis. The real world example is a sliders that used to support drawers.
#[derive(Clone, Debug, Visit, PartialEq, Reflect)]
pub struct PrismaticJoint {
    /// Whether linear limits along local X axis of the joint are enabled or not. Default is `false`
    #[reflect(
        description = "Whether linear limits along local X axis of the joint are enabled or not."
    )]
    #[visit(optional)] // Backward compatibility
    pub limits_enabled: bool,

    /// Allowed linear distance range along local X axis of the joint.
    #[reflect(description = "Allowed linear distance range along local X axis of the joint.")]
    #[visit(optional)] // Backward compatibility
    pub limits: Range<f32>,
}

impl Default for PrismaticJoint {
    fn default() -> Self {
        Self {
            limits_enabled: false,
            limits: -std::f32::consts::PI..std::f32::consts::PI,
        }
    }
}

/// The exact kind of the joint.
#[derive(Clone, Debug, PartialEq, Visit, Reflect, AsRefStr, EnumString, VariantNames)]
pub enum JointParams {
    /// See [`BallJoint`] for more info.
    BallJoint(BallJoint),
    /// See [`FixedJoint`] for more info.
    FixedJoint(FixedJoint),
    /// See [`PrismaticJoint`] for more info.
    PrismaticJoint(PrismaticJoint),
}

uuid_provider!(JointParams = "e1fa2015-3ea3-47bb-8ad3-d408559c9643");

impl Default for JointParams {
    fn default() -> Self {
        Self::BallJoint(Default::default())
    }
}

#[derive(Visit, Reflect, Debug, Clone, Default)]
pub(crate) struct LocalFrame {
    pub position: Vector2<f32>,
    pub rotation: UnitComplex<f32>,
}

impl LocalFrame {
    pub fn new(isometry: &Isometry2<f32>) -> Self {
        Self {
            position: isometry.translation.vector,
            rotation: isometry.rotation,
        }
    }
}

#[derive(Visit, Reflect, Debug, Clone, Default)]
pub(crate) struct JointLocalFrames {
    pub body1: LocalFrame,
    pub body2: LocalFrame,
}

impl JointLocalFrames {
    pub fn new(isometry1: &Isometry2<f32>, isometry2: &Isometry2<f32>) -> Self {
        Self {
            body1: LocalFrame::new(isometry1),
            body2: LocalFrame::new(isometry2),
        }
    }
}

/// Joint is used to restrict motion of two rigid bodies. There are numerous examples of joints in
/// real life: door hinge, ball joints in human arms, etc.
#[derive(Visit, Reflect, Debug)]
pub struct Joint {
    base: Base,

    #[reflect(setter = "set_params")]
    pub(crate) params: InheritableVariable<JointParams>,

    #[reflect(setter = "set_body1")]
    pub(crate) body1: InheritableVariable<Handle<Node>>,

    #[reflect(setter = "set_body2")]
    pub(crate) body2: InheritableVariable<Handle<Node>>,

    #[visit(optional)] // Backward compatibility
    #[reflect(setter = "set_contacts_enabled")]
    pub(crate) contacts_enabled: InheritableVariable<bool>,

    #[visit(optional)]
    #[reflect(hidden)]
    pub(crate) local_frames: RefCell<Option<JointLocalFrames>>,

    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) native: Cell<ImpulseJointHandle>,
}

impl Default for Joint {
    fn default() -> Self {
        Self {
            base: Default::default(),
            params: Default::default(),
            body1: Default::default(),
            body2: Default::default(),
            local_frames: Default::default(),
            contacts_enabled: InheritableVariable::new_modified(true),
            // Do not copy. The copy will have its own native representation.
            native: Cell::new(ImpulseJointHandle::invalid()),
        }
    }
}

impl Deref for Joint {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Joint {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Clone for Joint {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            params: self.params.clone(),
            body1: self.body1.clone(),
            body2: self.body2.clone(),
            local_frames: self.local_frames.clone(),
            contacts_enabled: self.contacts_enabled.clone(),
            native: Cell::new(ImpulseJointHandle::invalid()),
        }
    }
}

impl TypeUuidProvider for Joint {
    fn type_uuid() -> Uuid {
        uuid!("b8d66eda-b69f-4c57-80ba-d76665573565")
    }
}

impl Joint {
    /// Sets new parameters of the joint.
    pub fn set_params(&mut self, params: JointParams) -> JointParams {
        self.params.set_value_and_mark_modified(params)
    }

    /// Returns a shared reference to the current joint parameters.
    pub fn params(&self) -> &JointParams {
        &self.params
    }

    /// Returns a mutable reference to the current joint parameters. Obtaining the mutable reference
    /// will force the engine to do additional calculations to reflect changes to the physics engine.
    pub fn params_mut(&mut self) -> &mut JointParams {
        self.params.get_value_mut_and_mark_modified()
    }

    /// Sets the first body of the joint. The handle should point to the RigidBody node, otherwise
    /// the joint will have no effect!
    pub fn set_body1(&mut self, handle: Handle<Node>) -> Handle<Node> {
        self.body1.set_value_and_mark_modified(handle)
    }

    /// Returns current first body of the joint.
    pub fn body1(&self) -> Handle<Node> {
        *self.body1
    }

    /// Sets the second body of the joint. The handle should point to the RigidBody node, otherwise
    /// the joint will have no effect!
    pub fn set_body2(&mut self, handle: Handle<Node>) -> Handle<Node> {
        self.body2.set_value_and_mark_modified(handle)
    }

    /// Returns current second body of the joint.
    pub fn body2(&self) -> Handle<Node> {
        *self.body2
    }

    /// Sets whether the connected bodies should ignore collisions with each other or not.  
    pub fn set_contacts_enabled(&mut self, enabled: bool) -> bool {
        self.contacts_enabled.set_value_and_mark_modified(enabled)
    }

    /// Returns true if contacts between connected bodies is enabled, false - otherwise.
    pub fn is_contacts_enabled(&self) -> bool {
        *self.contacts_enabled
    }
}

impl NodeTrait for Joint {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn on_removed_from_graph(&mut self, graph: &mut Graph) {
        graph.physics2d.remove_joint(self.native.get());
        self.native.set(ImpulseJointHandle::invalid());

        Log::info(format!(
            "Native joint 2D was removed for node: {}",
            self.name()
        ));
    }

    fn sync_native(&self, self_handle: Handle<Node>, context: &mut SyncContext) {
        context
            .physics2d
            .sync_to_joint_node(context.nodes, self_handle, self);
    }

    fn sync_transform(&self, new_global_transform: &Matrix4<f32>, _context: &mut SyncContext) {
        if !m4x4_approx_eq(new_global_transform, &self.global_transform()) {
            self.local_frames.borrow_mut().take();
        }
    }

    fn validate(&self, scene: &Scene) -> Result<(), String> {
        if let Some(body1) = scene.graph.try_get(self.body1()) {
            if body1.query_component_ref::<RigidBody>().is_none() {
                return Err("First body of 2D Joint must be an \
                    instance of 2D Rigid Body!"
                    .to_string());
            }
        } else {
            return Err("2D Joint has invalid or unassigned handle to a \
            first body, the joint will not operate!"
                .to_string());
        }

        if let Some(body2) = scene.graph.try_get(self.body2()) {
            if body2.query_component_ref::<RigidBody>().is_none() {
                return Err("Second body of 2D Joint must be an instance \
                    of 2D Rigid Body!"
                    .to_string());
            }
        } else {
            return Err("2D Joint has invalid or unassigned handle to a \
            second body, the joint will not operate!"
                .to_string());
        }

        Ok(())
    }
}

/// Joint builder allows you to build Joint node in a declarative manner.
pub struct JointBuilder {
    base_builder: BaseBuilder,
    params: JointParams,
    body1: Handle<Node>,
    body2: Handle<Node>,
    contacts_enabled: bool,
}

impl JointBuilder {
    /// Creates a new joint builder instance.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            params: Default::default(),
            body1: Default::default(),
            body2: Default::default(),
            contacts_enabled: true,
        }
    }

    /// Sets desired joint parameters which defines exact type of the joint.
    pub fn with_params(mut self, params: JointParams) -> Self {
        self.params = params;
        self
    }

    /// Sets desired first body of the joint. This handle should be a handle to rigid body node,
    /// otherwise joint will have no effect!
    pub fn with_body1(mut self, body1: Handle<Node>) -> Self {
        self.body1 = body1;
        self
    }

    /// Sets desired second body of the joint. This handle should be a handle to rigid body node,
    /// otherwise joint will have no effect!
    pub fn with_body2(mut self, body2: Handle<Node>) -> Self {
        self.body2 = body2;
        self
    }

    /// Sets whether the connected bodies should ignore collisions with each other or not.  
    pub fn with_contacts_enabled(mut self, enabled: bool) -> Self {
        self.contacts_enabled = enabled;
        self
    }

    /// Creates new Joint node, but does not add it to the graph.
    pub fn build_joint(self) -> Joint {
        Joint {
            base: self.base_builder.build_base(),
            params: self.params.into(),
            body1: self.body1.into(),
            body2: self.body2.into(),
            local_frames: Default::default(),
            contacts_enabled: self.contacts_enabled.into(),
            native: Cell::new(ImpulseJointHandle::invalid()),
        }
    }

    /// Creates new Joint node, but does not add it to the graph.
    pub fn build_node(self) -> Node {
        Node::new(self.build_joint())
    }

    /// Creates new Joint node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
