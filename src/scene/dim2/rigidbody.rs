//! Rigid body is a physics entity that responsible for the dynamics and kinematics of the solid.
//!
//! # Common problems
//!
//! **Q:** Rigid body is "stuck".
//! **A:** Most likely the rigid body is "sleeping", in this case it must be activated manually, it is
//! most common problem with rigid bodies that controlled manually from code. They must be activated
//! using [`RigidBody::wake_up`]. By default any external action does **not** wakes up rigid body.
//! You can also explicitly tell to rigid body that it cannot sleep, by calling
//! [`RigidBody::set_can_sleep`] with `false` value.
use crate::scene::node::{NodeTrait, SyncContext, UpdateContext};
use crate::utils::log::Log;
use crate::{
    core::{
        algebra::Vector2,
        inspect::{Inspect, PropertyInfo},
        parking_lot::Mutex,
        pool::Handle,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    impl_directly_inheritable_entity_trait,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
        rigidbody::RigidBodyType,
        variable::{InheritError, TemplateVariable},
        DirectlyInheritableEntity,
    },
};
use fxhash::FxHashMap;
use fyrox_core::algebra::Matrix4;
use fyrox_core::math::aabb::AxisAlignedBoundingBox;
use fyrox_core::math::m4x4_approx_eq;
use fyrox_core::uuid::Uuid;
use rapier2d::prelude::RigidBodyHandle;
use std::str::FromStr;
use std::{
    cell::Cell,
    collections::VecDeque,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub(crate) enum ApplyAction {
    Force(Vector2<f32>),
    Torque(f32),
    ForceAtPoint {
        force: Vector2<f32>,
        point: Vector2<f32>,
    },
    Impulse(Vector2<f32>),
    TorqueImpulse(f32),
    ImpulseAtPoint {
        impulse: Vector2<f32>,
        point: Vector2<f32>,
    },
    WakeUp,
}

/// Rigid body is a physics entity that responsible for the dynamics and kinematics of the solid.
/// Use this node when you need to simulate real-world physics in your game.
///
/// # Sleeping
///
/// Rigid body that does not move for some time will go asleep. This means that the body will not
/// move unless it is woken up by some other moving body. This feature allows to save CPU resources.
#[derive(Visit, Inspect)]
pub struct RigidBody {
    base: Base,

    #[inspect(getter = "Deref::deref")]
    pub(crate) lin_vel: TemplateVariable<Vector2<f32>>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) ang_vel: TemplateVariable<f32>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) lin_damping: TemplateVariable<f32>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) ang_damping: TemplateVariable<f32>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) body_type: TemplateVariable<RigidBodyType>,

    #[inspect(min_value = 0.0, step = 0.05, getter = "Deref::deref")]
    pub(crate) mass: TemplateVariable<f32>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) rotation_locked: TemplateVariable<bool>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) translation_locked: TemplateVariable<bool>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) ccd_enabled: TemplateVariable<bool>,

    #[inspect(getter = "Deref::deref")]
    pub(crate) can_sleep: TemplateVariable<bool>,

    #[inspect(getter = "Deref::deref")]
    #[visit(optional)] // Backward compatibility
    pub(crate) dominance: TemplateVariable<i8>,

    #[inspect(getter = "Deref::deref")]
    #[visit(optional)] // Backward compatibility
    pub(crate) gravity_scale: TemplateVariable<f32>,

    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) sleeping: bool,

    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) native: Cell<RigidBodyHandle>,

    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) actions: Mutex<VecDeque<ApplyAction>>,
}

impl_directly_inheritable_entity_trait!(RigidBody;
    lin_vel,
    ang_vel,
    lin_damping,
    ang_damping,
    body_type,
    mass,
    rotation_locked,
    translation_locked,
    ccd_enabled,
    can_sleep,
    dominance,
    gravity_scale
);

impl Debug for RigidBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "RigidBody")
    }
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            base: Default::default(),
            lin_vel: Default::default(),
            ang_vel: Default::default(),
            lin_damping: Default::default(),
            ang_damping: Default::default(),
            sleeping: Default::default(),
            body_type: TemplateVariable::new(RigidBodyType::Dynamic),
            mass: TemplateVariable::new(1.0),
            rotation_locked: Default::default(),
            translation_locked: Default::default(),
            ccd_enabled: Default::default(),
            can_sleep: TemplateVariable::new(true),
            dominance: Default::default(),
            gravity_scale: TemplateVariable::new(1.0),
            native: Cell::new(RigidBodyHandle::invalid()),
            actions: Default::default(),
        }
    }
}

impl Deref for RigidBody {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for RigidBody {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Clone for RigidBody {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            lin_vel: self.lin_vel.clone(),
            ang_vel: self.ang_vel.clone(),
            lin_damping: self.lin_damping.clone(),
            ang_damping: self.ang_damping.clone(),
            sleeping: self.sleeping,
            body_type: self.body_type.clone(),
            mass: self.mass.clone(),
            rotation_locked: self.rotation_locked.clone(),
            translation_locked: self.translation_locked.clone(),
            ccd_enabled: self.ccd_enabled.clone(),
            can_sleep: self.can_sleep.clone(),
            dominance: self.dominance.clone(),
            gravity_scale: self.gravity_scale.clone(),
            // Do not copy.
            native: Cell::new(RigidBodyHandle::invalid()),
            actions: Default::default(),
        }
    }
}

impl RigidBody {
    pub fn type_uuid() -> Uuid {
        Uuid::from_str("0b242335-75a4-4c65-9685-3e82a8979047").unwrap()
    }

    /// Sets new linear velocity of the rigid body. Changing this parameter will wake up the rigid
    /// body!
    pub fn set_lin_vel(&mut self, lin_vel: Vector2<f32>) {
        self.lin_vel.set(lin_vel);
    }

    /// Returns current linear velocity of the rigid body.
    pub fn lin_vel(&self) -> Vector2<f32> {
        *self.lin_vel
    }

    /// Sets new angular velocity of the rigid body. Changing this parameter will wake up the rigid
    /// body!
    pub fn set_ang_vel(&mut self, ang_vel: f32) {
        self.ang_vel.set(ang_vel);
    }

    /// Returns current angular velocity of the rigid body.
    pub fn ang_vel(&self) -> f32 {
        *self.ang_vel
    }

    /// Sets _additional_ mass of the rigid body. It is called additional because real mass is defined
    /// by colliders attached to the body and their density and volume.
    pub fn set_mass(&mut self, mass: f32) {
        self.mass.set(mass);
    }

    /// Returns _additional_ mass of the rigid body.
    pub fn mass(&self) -> f32 {
        *self.mass
    }

    /// Sets angular damping of the rigid body. Angular damping will decrease angular velocity over
    /// time. Default is zero.
    pub fn set_ang_damping(&mut self, damping: f32) {
        self.ang_damping.set(damping);
    }

    /// Returns current angular damping.
    pub fn ang_damping(&self) -> f32 {
        *self.ang_damping
    }

    /// Sets linear damping of the rigid body. Linear damping will decrease linear velocity over
    /// time. Default is zero.
    pub fn set_lin_damping(&mut self, damping: f32) {
        self.lin_damping.set(damping);
    }

    /// Returns current linear damping.
    pub fn lin_damping(&self) -> f32 {
        *self.lin_damping
    }

    /// Locks rotations
    pub fn lock_rotations(&mut self, state: bool) {
        self.rotation_locked.set(state);
    }

    /// Returns true if rotation is locked, false - otherwise.
    pub fn is_rotation_locked(&self) -> bool {
        *self.rotation_locked
    }

    /// Locks translation in world coordinates.
    pub fn lock_translation(&mut self, state: bool) {
        self.translation_locked.set(state);
    }

    /// Returns true if translation is locked, false - otherwise.    
    pub fn is_translation_locked(&self) -> bool {
        *self.translation_locked
    }

    /// Sets new body type. See [`RigidBodyType`] for more info.
    pub fn set_body_type(&mut self, body_type: RigidBodyType) {
        self.body_type.set(body_type);
    }

    /// Returns current body type.
    pub fn body_type(&self) -> RigidBodyType {
        *self.body_type
    }

    /// Returns true if the rigid body is sleeping (temporarily excluded from simulation to save
    /// resources), false - otherwise.
    pub fn is_sleeping(&self) -> bool {
        self.sleeping
    }

    /// Returns true if continuous collision detection is enabled, false - otherwise.
    pub fn is_ccd_enabled(&self) -> bool {
        *self.ccd_enabled
    }

    /// Enables or disables continuous collision detection. CCD is very useful for fast moving objects
    /// to prevent accidental penetrations on high velocities.
    pub fn enable_ccd(&mut self, enable: bool) {
        self.ccd_enabled.set(enable);
    }

    /// Sets a gravity scale coefficient. Zero can be used to disable gravity.
    pub fn set_gravity_scale(&mut self, scale: f32) {
        self.gravity_scale.set(scale);
    }

    /// Returns current gravity scale coefficient.
    pub fn gravity_scale(&self) -> f32 {
        *self.gravity_scale
    }

    /// Sets dominance group of the rigid body. A rigid body with higher dominance group will not
    /// be affected by an object with lower dominance group (it will behave like it has an infinite
    /// mass). This is very importance feature for character physics in games, you can set highest
    /// dominance group to the player, and it won't be affected by any external forces.
    pub fn set_dominance(&mut self, dominance: i8) {
        self.dominance.set(dominance);
    }

    /// Returns current dominance group.
    pub fn dominance(&self) -> i8 {
        *self.dominance
    }

    /// Applies a force at the center-of-mass of this rigid-body. The force will be applied in the
    /// next simulation step. This does nothing on non-dynamic bodies.
    pub fn apply_force(&mut self, force: Vector2<f32>) {
        self.actions.get_mut().push_back(ApplyAction::Force(force))
    }

    /// Applies a torque at the center-of-mass of this rigid-body. The torque will be applied in
    /// the next simulation step. This does nothing on non-dynamic bodies.
    pub fn apply_torque(&mut self, torque: f32) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::Torque(torque))
    }

    /// Applies a force at the given world-space point of this rigid-body. The force will be applied
    /// in the next simulation step. This does nothing on non-dynamic bodies.
    pub fn apply_force_at_point(&mut self, force: Vector2<f32>, point: Vector2<f32>) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::ForceAtPoint { force, point })
    }

    /// Applies an impulse at the center-of-mass of this rigid-body. The impulse is applied right
    /// away, changing the linear velocity. This does nothing on non-dynamic bodies.
    pub fn apply_impulse(&mut self, impulse: Vector2<f32>) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::Impulse(impulse))
    }

    /// Applies an angular impulse at the center-of-mass of this rigid-body. The impulse is applied
    /// right away, changing the angular velocity. This does nothing on non-dynamic bodies.
    pub fn apply_torque_impulse(&mut self, torque_impulse: f32) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::TorqueImpulse(torque_impulse))
    }

    /// Applies an impulse at the given world-space point of this rigid-body. The impulse is applied
    /// right away, changing the linear and/or angular velocities. This does nothing on non-dynamic
    /// bodies.
    pub fn apply_impulse_at_point(&mut self, impulse: Vector2<f32>, point: Vector2<f32>) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::ImpulseAtPoint { impulse, point })
    }

    /// Sets whether the rigid body can sleep or not. If `false` is passed, it _automatically_ wake
    /// up rigid body.
    pub fn set_can_sleep(&mut self, can_sleep: bool) {
        self.can_sleep.set(can_sleep);
    }

    /// Returns true if the rigid body can sleep, false - otherwise.
    pub fn is_can_sleep(&self) -> bool {
        *self.can_sleep
    }

    /// Wakes up rigid body, forcing it to return to participate in the simulation.
    pub fn wake_up(&mut self) {
        self.actions.get_mut().push_back(ApplyAction::WakeUp)
    }

    pub(crate) fn need_sync_model(&self) -> bool {
        self.lin_vel.need_sync()
            || self.ang_vel.need_sync()
            || self.lin_damping.need_sync()
            || self.ang_damping.need_sync()
            || self.body_type.need_sync()
            || self.mass.need_sync()
            || self.rotation_locked.need_sync()
            || self.translation_locked.need_sync()
            || self.ccd_enabled.need_sync()
            || self.can_sleep.need_sync()
            || self.dominance.need_sync()
            || self.gravity_scale.need_sync()
    }
}

impl NodeTrait for RigidBody {
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

    fn restore_resources(&mut self, _resource_manager: ResourceManager) {}

    fn remap_handles(&mut self, old_new_mapping: &FxHashMap<Handle<Node>, Handle<Node>>) {
        self.base.remap_handles(old_new_mapping);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn clean_up(&mut self, graph: &mut Graph) {
        graph.physics2d.remove_body(self.native.get());

        Log::info(format!(
            "Native rigid body was removed for node: {}",
            self.name()
        ));
    }

    fn sync_native(&self, self_handle: Handle<Node>, context: &mut SyncContext) {
        context.physics2d.sync_to_rigid_body_node(self_handle, self);
    }

    fn sync_transform(&self, new_global_transform: &Matrix4<f32>, context: &mut SyncContext) {
        if !m4x4_approx_eq(&new_global_transform, &self.global_transform()) {
            context
                .physics2d
                .set_rigid_body_position(self, &new_global_transform);
        }
    }

    fn update(&mut self, context: &mut UpdateContext) -> bool {
        context
            .physics2d
            .sync_rigid_body_node(self, context.nodes[self.parent].global_transform());

        self.base.update_lifetime(context.dt)
    }
}

/// Allows you to create rigid body in declarative manner.
pub struct RigidBodyBuilder {
    base_builder: BaseBuilder,
    lin_vel: Vector2<f32>,
    ang_vel: f32,
    lin_damping: f32,
    ang_damping: f32,
    sleeping: bool,
    body_type: RigidBodyType,
    mass: f32,
    rotation_locked: bool,
    translation_locked: bool,
    ccd_enabled: bool,
    can_sleep: bool,
    dominance: i8,
    gravity_scale: f32,
}

impl RigidBodyBuilder {
    /// Creates new rigid body builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            lin_vel: Default::default(),
            ang_vel: Default::default(),
            lin_damping: 0.0,
            ang_damping: 0.0,
            sleeping: false,
            body_type: RigidBodyType::Dynamic,
            mass: 1.0,
            rotation_locked: false,
            translation_locked: false,
            ccd_enabled: false,
            can_sleep: true,
            dominance: 0,
            gravity_scale: 1.0,
        }
    }

    /// Sets the desired body type.
    pub fn with_body_type(mut self, body_type: RigidBodyType) -> Self {
        self.body_type = body_type;
        self
    }

    /// Sets the desired _additional_ mass of the body.
    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    /// Sets whether continuous collision detection should be enabled or not.
    pub fn with_ccd_enabled(mut self, enabled: bool) -> Self {
        self.ccd_enabled = enabled;
        self
    }

    /// Sets desired linear velocity.
    pub fn with_lin_vel(mut self, lin_vel: Vector2<f32>) -> Self {
        self.lin_vel = lin_vel;
        self
    }

    /// Sets desired angular velocity.
    pub fn with_ang_vel(mut self, ang_vel: f32) -> Self {
        self.ang_vel = ang_vel;
        self
    }

    /// Sets desired angular damping.
    pub fn with_ang_damping(mut self, ang_damping: f32) -> Self {
        self.ang_damping = ang_damping;
        self
    }

    /// Sets desired linear damping.
    pub fn with_lin_damping(mut self, lin_damping: f32) -> Self {
        self.lin_damping = lin_damping;
        self
    }

    /// Sets whether the rotation around X axis of the body should be locked or not.
    pub fn with_rotation_locked(mut self, rotation_locked: bool) -> Self {
        self.rotation_locked = rotation_locked;
        self
    }

    /// Sets whether the translation of the body should be locked or not.
    pub fn with_translation_locked(mut self, translation_locked: bool) -> Self {
        self.translation_locked = translation_locked;
        self
    }

    /// Sets desired dominance group.
    pub fn with_dominance(mut self, dominance: i8) -> Self {
        self.dominance = dominance;
        self
    }

    /// Sets desired gravity scale.
    pub fn with_gravity_scale(mut self, gravity_scale: f32) -> Self {
        self.gravity_scale = gravity_scale;
        self
    }

    /// Sets initial state of the body (sleeping or not).
    pub fn with_sleeping(mut self, sleeping: bool) -> Self {
        self.sleeping = sleeping;
        self
    }

    /// Sets whether rigid body can sleep or not.
    pub fn with_can_sleep(mut self, can_sleep: bool) -> Self {
        self.can_sleep = can_sleep;
        self
    }

    /// Creates RigidBody node but does not add it to the graph.
    pub fn build_rigid_body(self) -> RigidBody {
        RigidBody {
            base: self.base_builder.build_base(),
            lin_vel: self.lin_vel.into(),
            ang_vel: self.ang_vel.into(),
            lin_damping: self.lin_damping.into(),
            ang_damping: self.ang_damping.into(),
            sleeping: self.sleeping,
            body_type: self.body_type.into(),
            mass: self.mass.into(),
            rotation_locked: self.rotation_locked.into(),
            translation_locked: self.translation_locked.into(),
            ccd_enabled: self.ccd_enabled.into(),
            can_sleep: self.can_sleep.into(),
            dominance: self.dominance.into(),
            gravity_scale: self.gravity_scale.into(),
            native: Cell::new(RigidBodyHandle::invalid()),
            actions: Default::default(),
        }
    }

    /// Creates RigidBody node but does not add it to the graph.
    pub fn build_node(self) -> Node {
        Node::new(self.build_rigid_body())
    }

    /// Creates RigidBody node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::scene::node::NodeTrait;
    use crate::{
        core::algebra::Vector2,
        scene::{
            base::{test::check_inheritable_properties_equality, BaseBuilder},
            dim2::rigidbody::{RigidBodyBuilder, RigidBodyType},
            node::Node,
        },
    };

    #[test]
    fn test_rigid_body_2d_inheritance() {
        let parent = RigidBodyBuilder::new(BaseBuilder::new())
            .with_can_sleep(false)
            .with_mass(2.0)
            .with_sleeping(true)
            .with_rotation_locked(true)
            .with_ang_vel(1.0)
            .with_lin_vel(Vector2::new(2.0, 0.0))
            .with_ccd_enabled(true)
            .with_body_type(RigidBodyType::Static)
            .with_gravity_scale(0.5)
            .with_lin_damping(0.1)
            .with_ang_damping(0.1)
            .with_dominance(123)
            .with_translation_locked(true)
            .build_node();

        let mut child = RigidBodyBuilder::new(BaseBuilder::new()).build_rigid_body();

        child.inherit(&parent).unwrap();

        if let Node::RigidBody2D(parent) = parent {
            check_inheritable_properties_equality(&child, &parent);
        } else {
            unreachable!()
        }
    }
}
