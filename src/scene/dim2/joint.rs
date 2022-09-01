//! Joint is used to restrict motion of two rigid bodies.

use crate::{
    core::{
        algebra::Matrix4,
        inspect::{Inspect, PropertyInfo},
        math::{aabb::AxisAlignedBoundingBox, m4x4_approx_eq},
        pool::Handle,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        variable::TemplateVariable,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    scene::{
        base::{Base, BaseBuilder},
        graph::{map::NodeHandleMap, Graph},
        node::{Node, NodeTrait, SyncContext, TypeUuidProvider},
    },
    utils::log::Log,
};
use rapier2d::dynamics::ImpulseJointHandle;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut, Range},
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// Ball joint locks any translational moves between two objects on the axis between objects, but
/// allows rigid bodies to perform relative rotations. The real world example is a human shoulder,
/// pendulum, etc.
#[derive(Clone, Debug, Visit, PartialEq, Inspect, Reflect)]
pub struct BallJoint {
    /// Whether angular limits are enabled or not. Default is `false`
    #[inspect(description = "Whether angular limits are enabled or not.")]
    #[visit(optional)] // Backward compatibility
    pub limits_enabled: bool,

    /// Allowed angles range for the joint (in radians).
    #[inspect(description = "Allowed angles range for the joint (in radians).")]
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
#[derive(Clone, Debug, Default, Visit, PartialEq, Inspect, Reflect, Eq)]
pub struct FixedJoint;

/// Prismatic joint prevents any relative movement between two rigid-bodies, except for relative
/// translations along one axis. The real world example is a sliders that used to support drawers.
#[derive(Clone, Debug, Visit, PartialEq, Inspect, Reflect)]
pub struct PrismaticJoint {
    /// Whether linear limits along local X axis of the joint are enabled or not. Default is `false`
    #[inspect(
        description = "Whether linear limits along local X axis of the joint are enabled or not."
    )]
    #[visit(optional)] // Backward compatibility
    pub limits_enabled: bool,

    /// Allowed linear distance range along local X axis of the joint.
    #[inspect(description = "Allowed linear distance range along local X axis of the joint.")]
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
#[derive(
    Clone, Debug, PartialEq, Visit, Inspect, Reflect, AsRefStr, EnumString, EnumVariantNames,
)]
pub enum JointParams {
    /// See [`BallJoint`] for more info.
    BallJoint(BallJoint),
    /// See [`FixedJoint`] for more info.
    FixedJoint(FixedJoint),
    /// See [`PrismaticJoint`] for more info.
    PrismaticJoint(PrismaticJoint),
}

impl Default for JointParams {
    fn default() -> Self {
        Self::BallJoint(Default::default())
    }
}

/// Joint is used to restrict motion of two rigid bodies. There are numerous examples of joints in
/// real life: door hinge, ball joints in human arms, etc.
#[derive(Visit, Inspect, Reflect, Debug)]
pub struct Joint {
    base: Base,

    #[inspect(deref)]
    #[reflect(setter = "set_params")]
    pub(crate) params: TemplateVariable<JointParams>,

    #[inspect(deref)]
    #[reflect(setter = "set_body1")]
    pub(crate) body1: TemplateVariable<Handle<Node>>,

    #[inspect(deref)]
    #[reflect(setter = "set_body2")]
    pub(crate) body2: TemplateVariable<Handle<Node>>,

    #[inspect(deref)]
    #[visit(optional)] // Backward compatibility
    #[reflect(setter = "set_contacts_enabled")]
    pub(crate) contacts_enabled: TemplateVariable<bool>,

    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) native: Cell<ImpulseJointHandle>,

    #[visit(skip)]
    #[inspect(skip)]
    #[reflect(hidden)]
    pub(crate) need_rebind: Cell<bool>,
}

impl Default for Joint {
    fn default() -> Self {
        Self {
            base: Default::default(),
            params: Default::default(),
            body1: Default::default(),
            body2: Default::default(),
            contacts_enabled: TemplateVariable::new(true),
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
            contacts_enabled: self.contacts_enabled.clone(),
            native: Cell::new(ImpulseJointHandle::invalid()),
            need_rebind: Cell::new(true),
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
        self.params.set(params)
    }

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
    pub fn set_body1(&mut self, handle: Handle<Node>) -> Handle<Node> {
        self.body1.set(handle)
    }

    /// Returns current first body of the joint.
    pub fn body1(&self) -> Handle<Node> {
        *self.body1
    }

    /// Sets the second body of the joint. The handle should point to the RigidBody node, otherwise
    /// the joint will have no effect!
    pub fn set_body2(&mut self, handle: Handle<Node>) -> Handle<Node> {
        self.body2.set(handle)
    }

    /// Returns current second body of the joint.
    pub fn body2(&self) -> Handle<Node> {
        *self.body2
    }

    /// Sets whether the connected bodies should ignore collisions with each other or not.  
    pub fn set_contacts_enabled(&mut self, enabled: bool) -> bool {
        self.contacts_enabled.set(enabled)
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

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base.restore_resources(resource_manager);
    }

    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        self.base.remap_handles(old_new_mapping);

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
        graph.physics2d.remove_joint(self.native.get());

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
            contacts_enabled: self.contacts_enabled.into(),
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
    use crate::core::reflect::Reflect;
    use crate::core::variable::try_inherit_properties;
    use crate::scene::{
        base::{test::check_inheritable_properties_equality, BaseBuilder},
        dim2::joint::{BallJoint, Joint, JointBuilder, JointParams},
    };

    #[test]
    fn test_joint_2d_inheritance() {
        let parent = JointBuilder::new(BaseBuilder::new())
            .with_params(JointParams::BallJoint(BallJoint::default()))
            .build_node();

        let mut child = JointBuilder::new(BaseBuilder::new()).build_joint();

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        let parent = parent.cast::<Joint>().unwrap();

        check_inheritable_properties_equality(&child.base, &parent.base);
        check_inheritable_properties_equality(&child, parent);
    }
}
