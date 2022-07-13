//! Joint is used to restrict motion of two rigid bodies.

use crate::{
    core::{
        algebra::Matrix4,
        inspect::{Inspect, PropertyInfo},
        math::{aabb::AxisAlignedBoundingBox, m4x4_approx_eq},
        pool::Handle,
        uuid::{uuid, Uuid},
        variable::{InheritError, TemplateVariable},
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    impl_directly_inheritable_entity_trait,
    scene::{
        base::{Base, BaseBuilder},
        graph::{map::NodeHandleMap, Graph},
        node::{Node, NodeTrait, SyncContext, TypeUuidProvider},
        DirectlyInheritableEntity,
    },
    utils::log::Log,
};
use rapier3d::dynamics::ImpulseJointHandle;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut, Range},
};

/// Ball joint locks any translational moves between two objects on the axis between objects, but
/// allows rigid bodies to perform relative rotations. The real world example is a human shoulder,
/// pendulum, etc.
#[derive(Clone, Debug, Visit, PartialEq, Inspect)]
pub struct BallJoint {
    /// Whether X angular limits are enabled or not. Default is `false`
    #[inspect(description = "Whether X angular limits are enabled or not.")]
    #[visit(optional)] // Backward compatibility
    pub x_limits_enabled: bool,

    /// Allowed angle range around local X axis of the joint (in radians).
    #[inspect(description = "Allowed angle range around local X axis of the joint (in radians).")]
    #[visit(optional)] // Backward compatibility
    pub x_limits_angles: Range<f32>,

    /// Whether Y angular limits are enabled or not. Default is `false`
    #[inspect(description = "Whether Y angular limits are enabled or not.")]
    #[visit(optional)] // Backward compatibility
    pub y_limits_enabled: bool,

    /// Allowed angle range around local Y axis of the joint (in radians).
    #[inspect(description = "Allowed angle range around local Y axis of the joint (in radians).")]
    #[visit(optional)] // Backward compatibility
    pub y_limits_angles: Range<f32>,

    /// Whether Z angular limits are enabled or not. Default is `false`
    #[inspect(description = "Whether Z angular limits are enabled or not.")]
    #[visit(optional)] // Backward compatibility
    pub z_limits_enabled: bool,

    /// Allowed angle range around local Z axis of the joint (in radians).
    #[inspect(description = "Allowed angle range around local Z axis of the joint (in radians).")]
    #[visit(optional)] // Backward compatibility
    pub z_limits_angles: Range<f32>,
}

impl Default for BallJoint {
    fn default() -> Self {
        Self {
            x_limits_enabled: false,
            x_limits_angles: -std::f32::consts::PI..std::f32::consts::PI,
            y_limits_enabled: false,
            y_limits_angles: -std::f32::consts::PI..std::f32::consts::PI,
            z_limits_enabled: false,
            z_limits_angles: -std::f32::consts::PI..std::f32::consts::PI,
        }
    }
}

/// A fixed joint ensures that two rigid bodies does not move relative to each other. There is no
/// straightforward real-world example, but it can be thought as two bodies were "welded" together.
#[derive(Clone, Debug, Visit, PartialEq, Inspect, Default)]
pub struct FixedJoint;

/// Prismatic joint prevents any relative movement between two rigid-bodies, except for relative
/// translations along one axis. The real world example is a sliders that used to support drawers.
#[derive(Clone, Debug, Visit, PartialEq, Inspect)]
pub struct PrismaticJoint {
    /// Whether linear limits along local joint X axis are enabled or not. Default is `false`
    #[inspect(description = "Whether linear limits along local joint X axis are enabled or not.")]
    #[visit(optional)] // Backward compatibility
    pub limits_enabled: bool,

    /// The min an max relative position of the attached bodies along local X axis of the joint.
    #[inspect(
        description = "The min an max relative position of the attached bodies along local X axis of the joint."
    )]
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

/// Revolute joint prevents any relative movement between two rigid bodies, except relative rotation
/// along one axis. The real world example is wheels, fans, etc. It can also be used to simulate door
/// hinge.
#[derive(Clone, Debug, Visit, PartialEq, Inspect)]
pub struct RevoluteJoint {
    /// Whether angular limits around local X axis of the joint are enabled or not. Default is `false`
    #[inspect(
        description = "Whether angular limits around local X axis of the joint are enabled or not."
    )]
    #[visit(optional)] // Backward compatibility
    pub limits_enabled: bool,

    /// Allowed angle range around local X axis of the joint (in radians).
    #[inspect(description = "Allowed angle range around local X axis of the joint (in radians).")]
    #[visit(optional)] // Backward compatibility
    pub limits: Range<f32>,
}

impl Default for RevoluteJoint {
    fn default() -> Self {
        Self {
            limits_enabled: false,
            limits: -std::f32::consts::PI..std::f32::consts::PI,
        }
    }
}

/// The exact kind of the joint.
#[derive(Clone, Debug, PartialEq, Visit)]
pub enum JointParams {
    /// See [`BallJoint`] for more info.
    BallJoint(BallJoint),
    /// See [`FixedJoint`] for more info.
    FixedJoint(FixedJoint),
    /// See [`PrismaticJoint`] for more info.
    PrismaticJoint(PrismaticJoint),
    /// See [`RevoluteJoint`] for more info.
    RevoluteJoint(RevoluteJoint),
}

impl Inspect for JointParams {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            JointParams::BallJoint(v) => v.properties(),
            JointParams::FixedJoint(v) => v.properties(),
            JointParams::PrismaticJoint(v) => v.properties(),
            JointParams::RevoluteJoint(v) => v.properties(),
        }
    }
}

impl Default for JointParams {
    fn default() -> Self {
        Self::BallJoint(Default::default())
    }
}

/// Joint is used to restrict motion of two rigid bodies. There are numerous examples of joints in
/// real life: door hinge, ball joints in human arms, etc.
#[derive(Visit, Inspect, Debug)]
pub struct Joint {
    base: Base,

    #[inspect(getter = "Deref::deref")]
    pub(crate) params: TemplateVariable<JointParams>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) body1: TemplateVariable<Handle<Node>>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) body2: TemplateVariable<Handle<Node>>,

    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) native: Cell<ImpulseJointHandle>,

    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) need_rebind: Cell<bool>,
}

impl_directly_inheritable_entity_trait!(Joint;
    params,
    body1,
    body2
);

impl Default for Joint {
    fn default() -> Self {
        Self {
            base: Default::default(),
            params: Default::default(),
            body1: Default::default(),
            body2: Default::default(),
            native: Cell::new(ImpulseJointHandle::invalid()),
            need_rebind: Cell::new(true),
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
            native: Cell::new(ImpulseJointHandle::invalid()),
            need_rebind: Cell::new(true),
        }
    }
}

impl TypeUuidProvider for Joint {
    fn type_uuid() -> Uuid {
        uuid!("439d48f5-e3a3-4255-aa08-353c1ca42e3b")
    }
}

impl Joint {
    /// Returns a shared reference to the current joint parameters.
    pub fn params(&self) -> &JointParams {
        &self.params
    }

    /// Returns a mutable reference to the current joint parameters. Obtaining the mutable reference
    /// will force the engine to do additional calculations to reflect changes to the physics engine.
    pub fn params_mut(&mut self) -> &mut JointParams {
        self.params.get_mut()
    }

    /// Sets the first body of the joint. The handle should point to the RigidBody node, otherwise
    /// the joint will have no effect!
    pub fn set_body1(&mut self, handle: Handle<Node>) {
        self.body1.set(handle);
    }

    /// Returns current first body of the joint.
    pub fn body1(&self) -> Handle<Node> {
        *self.body1
    }

    /// Sets the second body of the joint. The handle should point to the RigidBody node, otherwise
    /// the joint will have no effect!
    pub fn set_body2(&mut self, handle: Handle<Node>) {
        self.body2.set(handle);
    }

    /// Returns current second body of the joint.
    pub fn body2(&self) -> Handle<Node> {
        *self.body2
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

    // Prefab inheritance resolving.
    fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        self.base.inherit_properties(parent)?;
        if let Some(parent) = parent.cast::<Self>() {
            self.try_inherit_self_properties(parent)?;
        }
        Ok(())
    }

    fn reset_inheritable_properties(&mut self) {
        self.base.reset_inheritable_properties();
        self.reset_self_inheritable_properties();
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base.restore_resources(resource_manager);
    }

    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        if !old_new_mapping.try_map_silent(&mut self.body1) {
            Log::warn(format!(
                "Unable to remap first body of a joint {}. Handle is {}!",
                self.name(),
                self.body1()
            ))
        }

        if !old_new_mapping.try_map_silent(&mut self.body2) {
            Log::warn(format!(
                "Unable to remap second body of a joint {}. Handle is {}!",
                self.name(),
                self.body2()
            ))
        }
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn clean_up(&mut self, graph: &mut Graph) {
        graph.physics.remove_joint(self.native.get());

        Log::info(format!(
            "Native joint was removed for node: {}",
            self.name()
        ));
    }

    fn sync_native(&self, self_handle: Handle<Node>, context: &mut SyncContext) {
        context
            .physics
            .sync_to_joint_node(context.nodes, self_handle, self);
    }

    fn sync_transform(&self, new_global_transform: &Matrix4<f32>, _context: &mut SyncContext) {
        if !m4x4_approx_eq(new_global_transform, &self.global_transform()) {
            self.need_rebind.set(true);
        }
    }
}

/// Joint builder allows you to build Joint node in a declarative manner.
pub struct JointBuilder {
    base_builder: BaseBuilder,
    params: JointParams,
    body1: Handle<Node>,
    body2: Handle<Node>,
}

impl JointBuilder {
    /// Creates a new joint builder instance.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            params: Default::default(),
            body1: Default::default(),
            body2: Default::default(),
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

    /// Creates new Joint node, but does not add it to the graph.
    pub fn build_joint(self) -> Joint {
        Joint {
            base: self.base_builder.build_base(),
            params: self.params.into(),
            body1: self.body1.into(),
            body2: self.body2.into(),
            native: Cell::new(ImpulseJointHandle::invalid()),
            need_rebind: Cell::new(true),
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

#[cfg(test)]
mod test {
    use crate::scene::{
        base::{test::check_inheritable_properties_equality, BaseBuilder},
        joint::{BallJoint, Joint, JointBuilder, JointParams},
        node::NodeTrait,
    };

    #[test]
    fn test_joint_inheritance() {
        let parent = JointBuilder::new(BaseBuilder::new())
            .with_params(JointParams::BallJoint(BallJoint::default()))
            .build_node();

        let mut child = JointBuilder::new(BaseBuilder::new()).build_joint();

        child.inherit(&parent).unwrap();

        let parent = parent.cast::<Joint>().unwrap();

        check_inheritable_properties_equality(&child.base, &parent.base);
        check_inheritable_properties_equality(&child, parent);
    }
}
