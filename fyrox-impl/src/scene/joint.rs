// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Joint is used to restrict motion of two rigid bodies.

use crate::scene::node::constructor::NodeConstructor;
use crate::{
    core::{
        algebra::Matrix4,
        log::Log,
        math::{aabb::AxisAlignedBoundingBox, m4x4_approx_eq},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, SyncContext},
        rigidbody::RigidBody,
        Scene,
    },
};
use fyrox_core::algebra::{Isometry3, Vector3};
use fyrox_core::uuid_provider;
use fyrox_graph::constructor::ConstructorProvider;
use fyrox_graph::{BaseSceneGraph, SceneGraph};
use rapier2d::na::UnitQuaternion;
use rapier3d::dynamics::ImpulseJointHandle;
use std::cell::RefCell;
use std::{
    cell::Cell,
    ops::{Deref, DerefMut, Range},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// Ball joint locks any translational moves between two objects on the axis between objects, but
/// allows rigid bodies to perform relative rotations. The real world example is a human shoulder,
/// pendulum, etc.
#[derive(Clone, Debug, Visit, PartialEq, Reflect)]
pub struct BallJoint {
    /// Whether X angular limits are enabled or not. Default is `false`
    #[visit(optional)] // Backward compatibility
    pub x_limits_enabled: bool,

    /// Allowed angle range around local X axis of the joint (in radians).
    #[visit(optional)] // Backward compatibility
    pub x_limits_angles: Range<f32>,

    /// Whether Y angular limits are enabled or not. Default is `false`
    #[visit(optional)] // Backward compatibility
    pub y_limits_enabled: bool,

    /// Allowed angle range around local Y axis of the joint (in radians).
    #[visit(optional)] // Backward compatibility
    pub y_limits_angles: Range<f32>,

    /// Whether Z angular limits are enabled or not. Default is `false`
    #[visit(optional)] // Backward compatibility
    pub z_limits_enabled: bool,

    /// Allowed angle range around local Z axis of the joint (in radians).
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
#[derive(Clone, Debug, Visit, PartialEq, Reflect, Default, Eq)]
pub struct FixedJoint;

/// Prismatic joint prevents any relative movement between two rigid-bodies, except for relative
/// translations along one axis. The real world example is a sliders that used to support drawers.
#[derive(Clone, Debug, Visit, PartialEq, Reflect)]
pub struct PrismaticJoint {
    /// Whether linear limits along local joint X axis are enabled or not. Default is `false`
    #[visit(optional)] // Backward compatibility
    pub limits_enabled: bool,

    /// The min an max relative position of the attached bodies along local X axis of the joint.
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
#[derive(Clone, Debug, Visit, PartialEq, Reflect)]
pub struct RevoluteJoint {
    /// Whether angular limits around local X axis of the joint are enabled or not. Default is `false`
    #[visit(optional)] // Backward compatibility
    pub limits_enabled: bool,

    /// Allowed angle range around local X axis of the joint (in radians).
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

/// Parameters that define how the joint motor will behave.
#[derive(Default, Clone, Debug, PartialEq, Visit, Reflect)]
pub struct JointMotorParams {
    /// The target velocity of the motor.
    pub target_vel: f32,
    /// The target position of the motor.
    pub target_pos: f32,
    /// The stiffness coefficient of the motor’s spring-like equation.
    pub stiffness: f32,
    /// The damping coefficient of the motor’s spring-like equation.
    pub damping: f32,
    /// The maximum force this motor can deliver.
    pub max_force: f32,
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
    /// See [`RevoluteJoint`] for more info.
    RevoluteJoint(RevoluteJoint),
}

uuid_provider!(JointParams = "a3e09303-9de4-4123-9492-05e27f29aaa3");

impl Default for JointParams {
    fn default() -> Self {
        Self::BallJoint(Default::default())
    }
}

#[derive(Visit, Reflect, Debug, Clone, Default)]
pub(crate) struct LocalFrame {
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
}

impl LocalFrame {
    pub fn new(isometry: &Isometry3<f32>) -> Self {
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
    pub fn new(isometry1: &Isometry3<f32>, isometry2: &Isometry3<f32>) -> Self {
        Self {
            body1: LocalFrame::new(isometry1),
            body2: LocalFrame::new(isometry2),
        }
    }
}

/// Joint is used to restrict motion of two rigid bodies. There are numerous examples of joints in
/// real life: door hinge, ball joints in human arms, etc.
#[derive(Visit, Reflect, Debug, ComponentProvider)]
#[reflect(derived_type = "Node")]
pub struct Joint {
    base: Base,

    #[reflect(setter = "set_params")]
    pub(crate) params: InheritableVariable<JointParams>,

    #[reflect(setter = "set_motor_params")]
    #[visit(optional)] // Backward compatibility
    pub(crate) motor_params: InheritableVariable<JointMotorParams>,

    #[reflect(setter = "set_body1")]
    pub(crate) body1: InheritableVariable<Handle<RigidBody>>,

    #[reflect(setter = "set_body2")]
    pub(crate) body2: InheritableVariable<Handle<RigidBody>>,

    #[reflect(setter = "set_contacts_enabled")]
    #[visit(optional)] // Backward compatibility
    pub(crate) contacts_enabled: InheritableVariable<bool>,

    #[reflect(setter = "set_auto_rebinding")]
    #[visit(optional)] // Backward compatibility
    pub(crate) auto_rebind: InheritableVariable<bool>,

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
            motor_params: Default::default(),
            body1: Default::default(),
            body2: Default::default(),
            contacts_enabled: InheritableVariable::new_modified(true),
            auto_rebind: true.into(),
            local_frames: Default::default(),
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
            motor_params: self.motor_params.clone(),
            body1: self.body1.clone(),
            body2: self.body2.clone(),
            contacts_enabled: self.contacts_enabled.clone(),
            local_frames: self.local_frames.clone(),
            // Do not copy. The copy will have its own native representation.
            auto_rebind: self.auto_rebind.clone(),
            native: Cell::new(ImpulseJointHandle::invalid()),
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
        self.params.get_value_mut_and_mark_modified()
    }

    /// Sets new joint parameters.
    pub fn set_params(&mut self, params: JointParams) -> JointParams {
        self.params.set_value_and_mark_modified(params)
    }

    /// Returns a shared reference to the current joint motor parameters.
    pub fn motor_params(&self) -> &JointMotorParams {
        &self.motor_params
    }

    /// Returns a mutable reference to the current joint motor parameters. Obtaining the mutable reference
    ///
    /// Recommend calling [`Self::set_motor_force_as_prismatic`] or [`Self::set_motor_torque_as_revolute`] for prismatic or revolute joints.
    ///
    /// Currently we do not support motor forces on more than one axis.
    ///
    /// If you have more complex needs, you may try to chain different joints together.
    /// # Notice
    /// If the joint is not RevoluteJoint or PrismaticJoint, modifying the motor parameters directly may lead to unexpected behavior.
    pub fn motor_params_mut(&mut self) -> &mut JointMotorParams {
        self.motor_params.get_value_mut_and_mark_modified()
    }

    /// Sets new joint motor parameters.
    ///
    /// Recommend calling [`Self::set_motor_force_as_prismatic`] or [`Self::set_motor_torque_as_revolute`] for prismatic or revolute joints.
    ///
    /// Currently we do not support motor forces on more than one axis.
    ///
    /// If you have more complex needs, you may try to chain different joints together.
    /// # Notice
    /// If the joint is not RevoluteJoint or PrismaticJoint, modifying the motor parameters directly may lead to unexpected behavior.
    pub fn set_motor_params(&mut self, motor_params: JointMotorParams) -> JointMotorParams {
        // to see how setting these params affect the rapier3d physics engine,
        // go to sync_native function in this file.
        self.motor_params.set_value_and_mark_modified(motor_params)
    }

    /// Sets the first body of the joint. The handle should point to the RigidBody node, otherwise
    /// the joint will have no effect!
    pub fn set_body1(&mut self, handle: Handle<RigidBody>) -> Handle<RigidBody> {
        self.body1.set_value_and_mark_modified(handle)
    }

    /// Returns current first body of the joint.
    pub fn body1(&self) -> Handle<RigidBody> {
        *self.body1
    }

    /// Sets the second body of the joint. The handle should point to the RigidBody node, otherwise
    /// the joint will have no effect!
    pub fn set_body2(&mut self, handle: Handle<RigidBody>) -> Handle<RigidBody> {
        self.body2.set_value_and_mark_modified(handle)
    }

    /// Returns current second body of the joint.
    pub fn body2(&self) -> Handle<RigidBody> {
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

    /// Sets whether the joint should automatically rebind two rigid bodies if the joint has changed its
    /// global position.
    pub fn set_auto_rebinding(&mut self, enabled: bool) -> bool {
        self.auto_rebind.set_value_and_mark_modified(enabled)
    }

    /// Returns true if automatic rebinding of the joint is enabled or not.
    pub fn is_auto_rebinding_enabled(&self) -> bool {
        *self.auto_rebind
    }

    /// Sets the motor force of the joint assuming it is a [`PrismaticJoint`].
    ///
    /// Call [`Self::disable_motor`] to properly stop the motor and set the joint free.
    /// # Arguments
    /// * `force` - The maximum force this motor can deliver.
    /// * `max_vel` - The target velocity of the motor.
    /// * `damping` - Controls how smoothly the motor will reach the target velocity. A higher damping value will result in a smoother transition to the target velocity.
    /// # Errors
    /// If the joint is not a [`PrismaticJoint`], this function will do nothing and return an Err.
    /// # Notice
    /// The rigid bodies attached to the joint may fall asleep anytime regardless whether the motor is enabled or not.
    ///
    /// To avoid this behavior, call this function periodically or call [`RigidBody::set_can_sleep`] on the rigid bodies with "false".
    pub fn set_motor_force_as_prismatic(
        &mut self,
        force: f32,
        max_vel: f32,
        damping: f32,
    ) -> Result<(), String> {
        let JointParams::PrismaticJoint(_) = self.params() else {
            return Err("Joint is not a PrismaticJoint".to_string());
        };
        let motor_params = JointMotorParams {
            target_vel: max_vel,
            target_pos: 0.0,
            stiffness: 0.0,
            damping,
            max_force: force,
        };
        // retrieving the mutable reference to the joint params will cause the engine to do additional calculations to reflect changes to the physics engine.
        self.set_motor_params(motor_params);
        Ok(())
    }

    /// Sets the motor torque of the joint assuming it is a [`RevoluteJoint`].
    ///
    /// Call [`Self::disable_motor`] to properly stop the motor and set the joint free.
    /// # Arguments
    /// * `torque` - The maximum torque this motor can deliver.
    /// * `max_angular_vel` - The target angular velocity of the motor.
    /// * `damping` - Controls how smoothly the motor will reach the target angular velocity. A higher damping value will result in a smoother transition to the target angular velocity.
    /// # Errors
    /// If the joint is not a [`RevoluteJoint`], this function will do nothing and return an Err.
    /// # Notice
    /// The rigid bodies attached to the joint may fall asleep anytime regardless whether the motor is enabled or not.
    ///
    /// To avoid this behavior, call this function periodically or call [`RigidBody::set_can_sleep`] on the rigid bodies with "false".
    pub fn set_motor_torque_as_revolute(
        &mut self,
        torque: f32,
        max_angular_vel: f32,
        damping: f32,
    ) -> Result<(), String> {
        let JointParams::RevoluteJoint(_) = self.params() else {
            return Err("Joint is not a RevoluteJoint".to_string());
        };
        let motor_params = JointMotorParams {
            target_vel: max_angular_vel,
            target_pos: 0.0,
            stiffness: 0.0,
            damping,
            max_force: torque,
        };
        // retrieving the mutable reference to the joint params will cause the engine to do additional calculations to reflect changes to the physics engine.
        self.set_motor_params(motor_params);
        Ok(())
    }

    /// Sets the motor target position of the joint assuming it is a [`PrismaticJoint`].
    ///
    /// After the joint reaches the target position, the joint will act as a spring with the specified stiffness and damping values.
    ///
    /// Call [`Self::disable_motor`] to stop the motor and remove the spring effect.
    /// # Arguments
    /// * `target_position` - The target position that the joint will try to reach, can be negative.
    /// * `stiffness` - Controls how fast the joint will try to reach the target position.
    /// * `max_force` - The maximum force this motor can deliver.
    /// * `damping` - Controls how much the joint will resist motion when it is moving towards the target position.
    /// # Errors
    /// If the joint is not a [`PrismaticJoint`], the function will do nothing and return an Err.
    /// # Notice
    /// The rigid bodies attached to the joint may fall asleep anytime regardless whether the motor is enabled or not.
    ///
    /// To avoid this behavior, call this function periodically or call [`RigidBody::set_can_sleep`] on the rigid bodies with "false".
    pub fn set_motor_target_position_as_prismatic(
        &mut self,
        target_position: f32,
        stiffness: f32,
        max_force: f32,
        damping: f32,
    ) -> Result<(), String> {
        let JointParams::PrismaticJoint(_) = self.params() else {
            return Err("Joint is not a PrismaticJoint".to_string());
        };
        let motor_params = JointMotorParams {
            target_vel: 0.0,
            target_pos: target_position,
            stiffness,
            damping,
            max_force,
        };
        // retrieving the mutable reference to the joint params will cause the engine to do additional calculations to reflect changes to the physics engine.
        self.set_motor_params(motor_params);
        Ok(())
    }

    /// Sets the motor target angle of the joint assuming it is a [`RevoluteJoint`].
    ///
    /// After the joint reaches the target angle, the joint will act as a spring with the specified stiffness and damping values.
    ///
    /// Call [`Self::disable_motor`] to stop the motor and remove the spring effect.
    /// # Arguments
    /// * `target_angle` - The target angle **in radians** that the joint will try to reach, can be negative. If the value is greater than 2π or less than -2π, the joint will turn multiple times to reach the target angle.
    /// * `stiffness` - Controls how fast the joint will try to reach the target angle.
    /// * `max_torque` - The maximum torque this motor can deliver.
    /// * `damping` - Controls how much the joint will resist motion when it is moving towards the target angle.
    /// # Errors
    /// If the joint is not a [`RevoluteJoint`], the function will do nothing and return an Err.
    /// # Notice
    /// The rigid bodies attached to the joint may fall asleep anytime regardless whether the motor is enabled or not.
    ///
    /// To avoid this behavior, call this function periodically or call [`RigidBody::set_can_sleep`] on the rigid bodies with "false".
    pub fn set_motor_target_angle_as_revolute(
        &mut self,
        target_angle: f32,
        stiffness: f32,
        max_torque: f32,
        damping: f32,
    ) -> Result<(), String> {
        let JointParams::RevoluteJoint(_) = self.params() else {
            return Err("Joint is not a RevoluteJoint".to_string());
        };
        let motor_params = JointMotorParams {
            target_vel: 0.0,
            target_pos: target_angle,
            stiffness,
            damping,
            max_force: max_torque,
        };
        // retrieving the mutable reference to the joint params will cause the engine to do additional calculations to reflect changes to the physics engine.
        self.set_motor_params(motor_params);
        Ok(())
    }

    /// Disables the motor of the joint assuming it is a [`RevoluteJoint`] or [`PrismaticJoint`].
    ///
    /// After this call, the joint will no longer apply any motor force or torque to the connected bodies.
    /// # Errors
    /// If the joint is not a [`RevoluteJoint`] or [`PrismaticJoint`], the function will do nothing and return an Err.
    pub fn disable_motor(&mut self) -> Result<(), String> {
        if !matches!(
            self.params(),
            JointParams::RevoluteJoint(_) | JointParams::PrismaticJoint(_)
        ) {
            return Err("Joint is not a RevoluteJoint or PrismaticJoint".to_string());
        }
        let motor_params = JointMotorParams {
            target_vel: 0.0,
            target_pos: 0.0,
            stiffness: 0.0,
            damping: 0.0,
            max_force: 0.0,
        };
        // retrieving the mutable reference to the joint params will cause the engine to do additional calculations to reflect changes to the physics engine.
        self.set_motor_params(motor_params);
        Ok(())
    }
}

impl ConstructorProvider<Node, Graph> for Joint {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_variant("Revolute Joint", |_| {
                JointBuilder::new(BaseBuilder::new().with_name("Revolute Joint"))
                    .with_params(JointParams::RevoluteJoint(Default::default()))
                    .build_node()
                    .into()
            })
            .with_variant("Ball Joint", |_| {
                JointBuilder::new(BaseBuilder::new().with_name("Ball Joint"))
                    .with_params(JointParams::BallJoint(Default::default()))
                    .build_node()
                    .into()
            })
            .with_variant("Prismatic Joint", |_| {
                JointBuilder::new(BaseBuilder::new().with_name("Prismatic Joint"))
                    .with_params(JointParams::PrismaticJoint(Default::default()))
                    .build_node()
                    .into()
            })
            .with_variant("Fixed Joint", |_| {
                JointBuilder::new(BaseBuilder::new().with_name("Fixed Joint"))
                    .with_params(JointParams::FixedJoint(Default::default()))
                    .build_node()
                    .into()
            })
            .with_group("Physics")
    }
}

impl NodeTrait for Joint {
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
        graph.physics.remove_joint(self.native.get());
        self.native.set(ImpulseJointHandle::invalid());

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

    fn on_global_transform_changed(
        &self,
        new_global_transform: &Matrix4<f32>,
        _context: &mut SyncContext,
    ) {
        if *self.auto_rebind && !m4x4_approx_eq(new_global_transform, &self.global_transform()) {
            self.local_frames.borrow_mut().take();
        }
    }

    fn validate(&self, scene: &Scene) -> Result<(), String> {
        if scene.graph.typed_ref(self.body1()).is_none() {
            return Err("3D Joint has invalid or unassigned handle to a \
            first body, the joint will not operate!"
                .to_string());
        }

        if scene.graph.typed_ref(self.body2()).is_none() {
            return Err("3D Joint has invalid or unassigned handle to a \
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
    motor_params: JointMotorParams,
    body1: Handle<RigidBody>,
    body2: Handle<RigidBody>,
    contacts_enabled: bool,
    auto_rebind: bool,
}

impl JointBuilder {
    /// Creates a new joint builder instance.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            params: Default::default(),
            motor_params: Default::default(),
            body1: Default::default(),
            body2: Default::default(),
            contacts_enabled: true,
            auto_rebind: true,
        }
    }

    /// Sets desired joint parameters which defines exact type of the joint.
    pub fn with_params(mut self, params: JointParams) -> Self {
        self.params = params;
        self
    }

    /// Set desired motor parameters which defines how the joint motor will behave.
    pub fn with_motor_params(mut self, motor_params: JointMotorParams) -> Self {
        self.motor_params = motor_params;
        self
    }

    /// Sets desired first body of the joint. This handle should be a handle to rigid body node,
    /// otherwise joint will have no effect!
    pub fn with_body1(mut self, body1: Handle<RigidBody>) -> Self {
        self.body1 = body1;
        self
    }

    /// Sets desired second body of the joint. This handle should be a handle to rigid body node,
    /// otherwise joint will have no effect!
    pub fn with_body2(mut self, body2: Handle<RigidBody>) -> Self {
        self.body2 = body2;
        self
    }

    /// Sets whether the connected bodies should ignore collisions with each other or not.
    pub fn with_contacts_enabled(mut self, enabled: bool) -> Self {
        self.contacts_enabled = enabled;
        self
    }

    /// Sets whether the joint should automatically rebind two rigid bodies if the joint has changed its
    /// global position.
    pub fn with_auto_rebinding_enabled(mut self, auto_rebind: bool) -> Self {
        self.auto_rebind = auto_rebind;
        self
    }

    /// Creates new Joint node, but does not add it to the graph.
    pub fn build_joint(self) -> Joint {
        Joint {
            base: self.base_builder.build_base(),
            params: self.params.into(),
            motor_params: self.motor_params.into(),
            body1: self.body1.into(),
            body2: self.body2.into(),
            contacts_enabled: self.contacts_enabled.into(),
            auto_rebind: self.auto_rebind.into(),
            local_frames: Default::default(),
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
