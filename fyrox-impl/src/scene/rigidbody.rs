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
use crate::{
    core::{
        algebra::{Matrix4, Vector3},
        log::Log,
        math::{aabb::AxisAlignedBoundingBox, m4x4_approx_eq},
        parking_lot::Mutex,
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    scene::{
        base::{Base, BaseBuilder},
        collider::Collider,
        graph::Graph,
        node::{Node, NodeTrait, SyncContext, UpdateContext},
        Scene,
    },
};
use fyrox_core::uuid_provider;
use fyrox_graph::{BaseSceneGraph, SceneGraph};
use rapier3d::{dynamics, prelude::RigidBodyHandle};
use std::{
    cell::Cell,
    collections::VecDeque,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};
use strum_macros::{AsRefStr, EnumString, VariantNames};

/// A set of possible types of rigid body.
#[derive(
    Copy, Clone, Debug, Reflect, Visit, PartialEq, Eq, Hash, AsRefStr, EnumString, VariantNames,
)]
#[repr(u32)]
pub enum RigidBodyType {
    /// Dynamic rigid bodies can be affected by external forces.
    Dynamic = 0,
    /// Static rigid bodies cannot be affected by external forces.
    Static = 1,
    /// Kinematic rigid body cannot be affected by external forces, but can push other rigid bodies.
    /// It also does not have any dynamic, you are able to control the position manually.
    KinematicPositionBased = 2,
    /// Kinematic rigid body cannot be affected by external forces, but can push other rigid bodies.
    /// It also does not have any dynamic, you are able to control the position by changing velocity.
    KinematicVelocityBased = 3,
}

uuid_provider!(RigidBodyType = "562d2907-1b41-483a-8ca2-12eebaff7f5d");

impl Default for RigidBodyType {
    fn default() -> Self {
        Self::Dynamic
    }
}

impl From<dynamics::RigidBodyType> for RigidBodyType {
    fn from(s: dynamics::RigidBodyType) -> Self {
        match s {
            dynamics::RigidBodyType::Dynamic => Self::Dynamic,
            dynamics::RigidBodyType::Fixed => Self::Static,
            dynamics::RigidBodyType::KinematicPositionBased => Self::KinematicPositionBased,
            dynamics::RigidBodyType::KinematicVelocityBased => Self::KinematicVelocityBased,
        }
    }
}

impl From<RigidBodyType> for rapier3d::dynamics::RigidBodyType {
    fn from(v: RigidBodyType) -> Self {
        match v {
            RigidBodyType::Dynamic => rapier3d::dynamics::RigidBodyType::Dynamic,
            RigidBodyType::Static => rapier3d::dynamics::RigidBodyType::Fixed,
            RigidBodyType::KinematicPositionBased => {
                rapier3d::dynamics::RigidBodyType::KinematicPositionBased
            }
            RigidBodyType::KinematicVelocityBased => {
                rapier3d::dynamics::RigidBodyType::KinematicVelocityBased
            }
        }
    }
}

impl From<RigidBodyType> for rapier2d::dynamics::RigidBodyType {
    fn from(v: RigidBodyType) -> Self {
        match v {
            RigidBodyType::Dynamic => rapier2d::dynamics::RigidBodyType::Dynamic,
            RigidBodyType::Static => rapier2d::dynamics::RigidBodyType::Fixed,
            RigidBodyType::KinematicPositionBased => {
                rapier2d::dynamics::RigidBodyType::KinematicPositionBased
            }
            RigidBodyType::KinematicVelocityBased => {
                rapier2d::dynamics::RigidBodyType::KinematicVelocityBased
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum ApplyAction {
    Force(Vector3<f32>),
    Torque(Vector3<f32>),
    ForceAtPoint {
        force: Vector3<f32>,
        point: Vector3<f32>,
    },
    Impulse(Vector3<f32>),
    TorqueImpulse(Vector3<f32>),
    ImpulseAtPoint {
        impulse: Vector3<f32>,
        point: Vector3<f32>,
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
#[derive(Visit, Reflect)]
pub struct RigidBody {
    base: Base,

    #[reflect(setter = "set_lin_vel")]
    pub(crate) lin_vel: InheritableVariable<Vector3<f32>>,

    #[reflect(setter = "set_ang_vel")]
    pub(crate) ang_vel: InheritableVariable<Vector3<f32>>,

    #[reflect(setter = "set_lin_damping")]
    pub(crate) lin_damping: InheritableVariable<f32>,

    #[reflect(setter = "set_ang_damping")]
    pub(crate) ang_damping: InheritableVariable<f32>,

    #[reflect(setter = "set_body_type")]
    pub(crate) body_type: InheritableVariable<RigidBodyType>,

    #[reflect(min_value = 0.0, step = 0.05)]
    #[reflect(setter = "set_mass")]
    pub(crate) mass: InheritableVariable<f32>,

    #[reflect(setter = "lock_x_rotations")]
    pub(crate) x_rotation_locked: InheritableVariable<bool>,

    #[reflect(setter = "lock_y_rotations")]
    pub(crate) y_rotation_locked: InheritableVariable<bool>,

    #[reflect(setter = "lock_z_rotations")]
    pub(crate) z_rotation_locked: InheritableVariable<bool>,

    #[reflect(setter = "lock_translation")]
    pub(crate) translation_locked: InheritableVariable<bool>,

    #[reflect(setter = "enable_ccd")]
    pub(crate) ccd_enabled: InheritableVariable<bool>,

    #[reflect(setter = "set_can_sleep")]
    pub(crate) can_sleep: InheritableVariable<bool>,

    #[reflect(setter = "set_dominance")]
    pub(crate) dominance: InheritableVariable<i8>,

    #[reflect(setter = "set_gravity_scale")]
    pub(crate) gravity_scale: InheritableVariable<f32>,

    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) sleeping: bool,
    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) reset_forces: Cell<bool>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) native: Cell<RigidBodyHandle>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) actions: Mutex<VecDeque<ApplyAction>>,
}

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
            body_type: InheritableVariable::new_modified(RigidBodyType::Dynamic),
            mass: InheritableVariable::new_modified(1.0),
            x_rotation_locked: Default::default(),
            y_rotation_locked: Default::default(),
            z_rotation_locked: Default::default(),
            translation_locked: Default::default(),
            ccd_enabled: Default::default(),
            can_sleep: InheritableVariable::new_modified(true),
            dominance: Default::default(),
            gravity_scale: InheritableVariable::new_modified(1.0),
            native: Cell::new(RigidBodyHandle::invalid()),
            actions: Default::default(),
            reset_forces: Default::default(),
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
            x_rotation_locked: self.x_rotation_locked.clone(),
            y_rotation_locked: self.y_rotation_locked.clone(),
            z_rotation_locked: self.z_rotation_locked.clone(),
            translation_locked: self.translation_locked.clone(),
            ccd_enabled: self.ccd_enabled.clone(),
            can_sleep: self.can_sleep.clone(),
            dominance: self.dominance.clone(),
            gravity_scale: self.gravity_scale.clone(),
            // Do not copy. The copy will have its own native representation.
            native: Cell::new(RigidBodyHandle::invalid()),
            actions: Default::default(),
            reset_forces: self.reset_forces.clone(),
        }
    }
}

impl TypeUuidProvider for RigidBody {
    fn type_uuid() -> Uuid {
        uuid!("4be15a7c-3566-49c4-bba8-2f4ccc57ffed")
    }
}

impl RigidBody {
    /// Sets new linear velocity of the rigid body. Changing this parameter will wake up the rigid
    /// body!
    pub fn set_lin_vel(&mut self, lin_vel: Vector3<f32>) -> Vector3<f32> {
        self.lin_vel.set_value_and_mark_modified(lin_vel)
    }

    /// Returns current linear velocity of the rigid body.
    pub fn lin_vel(&self) -> Vector3<f32> {
        *self.lin_vel
    }

    /// Sets new angular velocity of the rigid body. Changing this parameter will wake up the rigid
    /// body!
    pub fn set_ang_vel(&mut self, ang_vel: Vector3<f32>) -> Vector3<f32> {
        self.ang_vel.set_value_and_mark_modified(ang_vel)
    }

    /// Returns current angular velocity of the rigid body.
    pub fn ang_vel(&self) -> Vector3<f32> {
        *self.ang_vel
    }

    /// Sets _additional_ mass of the rigid body. It is called additional because real mass is defined
    /// by colliders attached to the body and their density and volume.
    pub fn set_mass(&mut self, mass: f32) -> f32 {
        self.mass.set_value_and_mark_modified(mass)
    }

    /// Returns _additional_ mass of the rigid body.
    pub fn mass(&self) -> f32 {
        *self.mass
    }

    /// Sets angular damping of the rigid body. Angular damping will decrease angular velocity over
    /// time. Default is zero.
    pub fn set_ang_damping(&mut self, damping: f32) -> f32 {
        self.ang_damping.set_value_and_mark_modified(damping)
    }

    /// Returns current angular damping.
    pub fn ang_damping(&self) -> f32 {
        *self.ang_damping
    }

    /// Sets linear damping of the rigid body. Linear damping will decrease linear velocity over
    /// time. Default is zero.
    pub fn set_lin_damping(&mut self, damping: f32) -> f32 {
        self.lin_damping.set_value_and_mark_modified(damping)
    }

    /// Returns current linear damping.
    pub fn lin_damping(&self) -> f32 {
        *self.lin_damping
    }

    /// Locks rotations around X axis in world coordinates.
    pub fn lock_x_rotations(&mut self, state: bool) -> bool {
        self.x_rotation_locked.set_value_and_mark_modified(state)
    }

    /// Returns true if rotation around X axis is locked, false - otherwise.
    pub fn is_x_rotation_locked(&self) -> bool {
        *self.x_rotation_locked
    }

    /// Locks rotations around Y axis in world coordinates.
    pub fn lock_y_rotations(&mut self, state: bool) -> bool {
        self.y_rotation_locked.set_value_and_mark_modified(state)
    }

    /// Returns true if rotation around Y axis is locked, false - otherwise.    
    pub fn is_y_rotation_locked(&self) -> bool {
        *self.y_rotation_locked
    }

    /// Locks rotations around Z axis in world coordinates.
    pub fn lock_z_rotations(&mut self, state: bool) -> bool {
        self.z_rotation_locked.set_value_and_mark_modified(state)
    }

    /// Returns true if rotation around Z axis is locked, false - otherwise.    
    pub fn is_z_rotation_locked(&self) -> bool {
        *self.z_rotation_locked
    }

    /// Locks or unlocks rotations around all axes at once.
    pub fn lock_rotations(&mut self, locked: bool) {
        self.x_rotation_locked.set_value_and_mark_modified(locked);
        self.y_rotation_locked.set_value_and_mark_modified(locked);
        self.z_rotation_locked.set_value_and_mark_modified(locked);
    }

    /// Locks translation in world coordinates.
    pub fn lock_translation(&mut self, state: bool) -> bool {
        self.translation_locked.set_value_and_mark_modified(state)
    }

    /// Returns true if translation is locked, false - otherwise.    
    pub fn is_translation_locked(&self) -> bool {
        *self.translation_locked
    }

    /// Sets new body type. See [`RigidBodyType`] for more info.
    pub fn set_body_type(&mut self, body_type: RigidBodyType) -> RigidBodyType {
        self.body_type.set_value_and_mark_modified(body_type)
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
    pub fn enable_ccd(&mut self, enable: bool) -> bool {
        self.ccd_enabled.set_value_and_mark_modified(enable)
    }

    /// Sets a gravity scale coefficient. Zero can be used to disable gravity.
    pub fn set_gravity_scale(&mut self, scale: f32) -> f32 {
        self.gravity_scale.set_value_and_mark_modified(scale)
    }

    /// Returns current gravity scale coefficient.
    pub fn gravity_scale(&self) -> f32 {
        *self.gravity_scale
    }

    /// Sets dominance group of the rigid body. A rigid body with higher dominance group will not
    /// be affected by an object with lower dominance group (it will behave like it has an infinite
    /// mass). This is very importance feature for character physics in games, you can set highest
    /// dominance group to the player, and it won't be affected by any external forces.
    pub fn set_dominance(&mut self, dominance: i8) -> i8 {
        self.dominance.set_value_and_mark_modified(dominance)
    }

    /// Returns current dominance group.
    pub fn dominance(&self) -> i8 {
        *self.dominance
    }

    /// Applies a force at the center-of-mass of this rigid-body. The force will be applied in the
    /// next simulation step. This does nothing on non-dynamic bodies.
    pub fn apply_force(&mut self, force: Vector3<f32>) {
        self.actions.get_mut().push_back(ApplyAction::Force(force))
    }

    /// Applies a torque at the center-of-mass of this rigid-body. The torque will be applied in
    /// the next simulation step. This does nothing on non-dynamic bodies.
    pub fn apply_torque(&mut self, torque: Vector3<f32>) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::Torque(torque))
    }

    /// Applies a force at the given world-space point of this rigid-body. The force will be applied
    /// in the next simulation step. This does nothing on non-dynamic bodies.
    pub fn apply_force_at_point(&mut self, force: Vector3<f32>, point: Vector3<f32>) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::ForceAtPoint { force, point })
    }

    /// Applies an impulse at the center-of-mass of this rigid-body. The impulse is applied right
    /// away, changing the linear velocity. This does nothing on non-dynamic bodies.
    pub fn apply_impulse(&mut self, impulse: Vector3<f32>) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::Impulse(impulse))
    }

    /// Applies an angular impulse at the center-of-mass of this rigid-body. The impulse is applied
    /// right away, changing the angular velocity. This does nothing on non-dynamic bodies.
    pub fn apply_torque_impulse(&mut self, torque_impulse: Vector3<f32>) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::TorqueImpulse(torque_impulse))
    }

    /// Applies an impulse at the given world-space point of this rigid-body. The impulse is applied
    /// right away, changing the linear and/or angular velocities. This does nothing on non-dynamic
    /// bodies.
    pub fn apply_impulse_at_point(&mut self, impulse: Vector3<f32>, point: Vector3<f32>) {
        self.actions
            .get_mut()
            .push_back(ApplyAction::ImpulseAtPoint { impulse, point })
    }

    /// Sets whether the rigid body can sleep or not. If `false` is passed, it _automatically_ wake
    /// up rigid body.
    pub fn set_can_sleep(&mut self, can_sleep: bool) -> bool {
        self.can_sleep.set_value_and_mark_modified(can_sleep)
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
            || self.x_rotation_locked.need_sync()
            || self.y_rotation_locked.need_sync()
            || self.z_rotation_locked.need_sync()
            || self.translation_locked.need_sync()
            || self.ccd_enabled.need_sync()
            || self.can_sleep.need_sync()
            || self.dominance.need_sync()
            || self.gravity_scale.need_sync()
            || self.reset_forces.get()
    }
}

impl NodeTrait for RigidBody {
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
        graph.physics.remove_body(self.native.get());
        self.native.set(RigidBodyHandle::invalid());

        Log::info(format!(
            "Native rigid body was removed for node: {}",
            self.name()
        ));
    }

    fn sync_native(&self, self_handle: Handle<Node>, context: &mut SyncContext) {
        context.physics.sync_to_rigid_body_node(self_handle, self);
    }

    fn sync_transform(&self, new_global_transform: &Matrix4<f32>, context: &mut SyncContext) {
        if !m4x4_approx_eq(new_global_transform, &self.global_transform()) {
            context
                .physics
                .set_rigid_body_position(self, new_global_transform);
        }
    }

    fn update(&mut self, context: &mut UpdateContext) {
        context.physics.sync_rigid_body_node(
            self,
            // Rigid body can be root node of a scene, in this case it does not have a parent.
            context
                .nodes
                .try_borrow(self.parent)
                .map(|p| p.global_transform())
                .unwrap_or_else(Matrix4::identity),
        );
    }

    fn validate(&self, scene: &Scene) -> Result<(), String> {
        for &child in self.children() {
            if scene.graph.try_get_of_type::<Collider>(child).is_some() {
                return Ok(());
            }
        }

        Err("The 3D rigid body must have at least one 3D collider as a \
        direct child node to work correctly!"
            .to_string())
    }
}

/// Allows you to create rigid body in declarative manner.
pub struct RigidBodyBuilder {
    base_builder: BaseBuilder,
    lin_vel: Vector3<f32>,
    ang_vel: Vector3<f32>,
    lin_damping: f32,
    ang_damping: f32,
    sleeping: bool,
    body_type: RigidBodyType,
    mass: f32,
    x_rotation_locked: bool,
    y_rotation_locked: bool,
    z_rotation_locked: bool,
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
            x_rotation_locked: false,
            y_rotation_locked: false,
            z_rotation_locked: false,
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
    pub fn with_lin_vel(mut self, lin_vel: Vector3<f32>) -> Self {
        self.lin_vel = lin_vel;
        self
    }

    /// Sets desired angular velocity.
    pub fn with_ang_vel(mut self, ang_vel: Vector3<f32>) -> Self {
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
    pub fn with_x_rotation_locked(mut self, x_rotation_locked: bool) -> Self {
        self.x_rotation_locked = x_rotation_locked;
        self
    }

    /// Sets whether the rotation around Y axis of the body should be locked or not.
    pub fn with_y_rotation_locked(mut self, y_rotation_locked: bool) -> Self {
        self.y_rotation_locked = y_rotation_locked;
        self
    }

    /// Sets whether the rotation around Z axis of the body should be locked or not.
    pub fn with_z_rotation_locked(mut self, z_rotation_locked: bool) -> Self {
        self.z_rotation_locked = z_rotation_locked;
        self
    }

    /// Sets whether the translation of the body should be locked or not.
    pub fn with_translation_locked(mut self, translation_locked: bool) -> Self {
        self.translation_locked = translation_locked;
        self
    }

    /// Locks or unlocks rotations of the rigid body.
    pub fn with_locked_rotations(mut self, locked: bool) -> Self {
        self.x_rotation_locked = locked;
        self.y_rotation_locked = locked;
        self.z_rotation_locked = locked;
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
            x_rotation_locked: self.x_rotation_locked.into(),
            y_rotation_locked: self.y_rotation_locked.into(),
            z_rotation_locked: self.z_rotation_locked.into(),
            translation_locked: self.translation_locked.into(),
            ccd_enabled: self.ccd_enabled.into(),
            can_sleep: self.can_sleep.into(),
            dominance: self.dominance.into(),
            gravity_scale: self.gravity_scale.into(),
            native: Cell::new(RigidBodyHandle::invalid()),
            actions: Default::default(),
            reset_forces: Default::default(),
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
