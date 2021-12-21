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
        algebra::Vector3,
        inspect::{Inspect, PropertyInfo},
        parking_lot::Mutex,
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::rapier::{dynamics, prelude::RigidBodyHandle},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use bitflags::bitflags;
use std::{
    cell::Cell,
    collections::VecDeque,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

/// A set of possible types of rigid body.
#[derive(Copy, Clone, Debug, Inspect, Visit, PartialEq, Eq, Hash)]
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

impl Default for RigidBodyType {
    fn default() -> Self {
        Self::Dynamic
    }
}

impl From<dynamics::RigidBodyType> for RigidBodyType {
    fn from(s: dynamics::RigidBodyType) -> Self {
        match s {
            dynamics::RigidBodyType::Dynamic => Self::Dynamic,
            dynamics::RigidBodyType::Static => Self::Static,
            dynamics::RigidBodyType::KinematicPositionBased => Self::KinematicPositionBased,
            dynamics::RigidBodyType::KinematicVelocityBased => Self::KinematicVelocityBased,
        }
    }
}

impl From<RigidBodyType> for dynamics::RigidBodyType {
    fn from(v: RigidBodyType) -> Self {
        match v {
            RigidBodyType::Dynamic => dynamics::RigidBodyType::Dynamic,
            RigidBodyType::Static => dynamics::RigidBodyType::Static,
            RigidBodyType::KinematicPositionBased => {
                dynamics::RigidBodyType::KinematicPositionBased
            }
            RigidBodyType::KinematicVelocityBased => {
                dynamics::RigidBodyType::KinematicVelocityBased
            }
        }
    }
}

bitflags! {
    pub(crate) struct RigidBodyChanges: u32 {
        const NONE = 0;
        const LIN_VEL = 0b0000_0001;
        const ANG_VEL = 0b0000_0010;
        const BODY_TYPE = 0b0000_0100;
        const ROTATION_LOCKED = 0b0000_1000;
        const TRANSLATION_LOCKED = 0b0001_0000;
        const MASS = 0b0010_0000;
        const ANG_DAMPING = 0b0100_0000;
        const LIN_DAMPING = 0b1000_0000;
        const CCD_STATE = 0b0001_0000_0000;
        const CAN_SLEEP = 0b0010_0000_0000;
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
#[derive(Visit, Inspect)]
pub struct RigidBody {
    base: Base,
    pub(crate) lin_vel: Vector3<f32>,
    pub(crate) ang_vel: Vector3<f32>,
    pub(crate) lin_damping: f32,
    pub(crate) ang_damping: f32,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) sleeping: bool,
    body_type: RigidBodyType,
    #[inspect(min_value = 0.0, step = 0.05)]
    mass: f32,
    x_rotation_locked: bool,
    y_rotation_locked: bool,
    z_rotation_locked: bool,
    translation_locked: bool,
    ccd_enabled: bool,
    can_sleep: bool,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) native: Cell<RigidBodyHandle>,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) changes: Cell<RigidBodyChanges>,
    #[visit(skip)]
    #[inspect(skip)]
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
            native: Cell::new(RigidBodyHandle::invalid()),
            changes: Cell::new(RigidBodyChanges::NONE),
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

impl RigidBody {
    /// Creates a raw copy of the RigidBody node. This method is for internal use only.
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            lin_vel: self.lin_vel,
            ang_vel: self.ang_vel,
            lin_damping: self.lin_damping,
            ang_damping: self.ang_damping,
            sleeping: self.sleeping,
            body_type: self.body_type,
            mass: self.mass,
            x_rotation_locked: self.x_rotation_locked,
            y_rotation_locked: self.y_rotation_locked,
            z_rotation_locked: self.z_rotation_locked,
            translation_locked: self.translation_locked,
            ccd_enabled: self.ccd_enabled,
            can_sleep: self.can_sleep,
            // Do not copy.
            native: Cell::new(RigidBodyHandle::invalid()),
            changes: Cell::new(RigidBodyChanges::NONE),
            actions: Default::default(),
        }
    }

    /// Sets new linear velocity of the rigid body. Changing this parameter will wake up the rigid
    /// body!
    pub fn set_lin_vel(&mut self, lin_vel: Vector3<f32>) {
        self.lin_vel = lin_vel;
        self.changes.get_mut().insert(RigidBodyChanges::LIN_VEL);
    }

    /// Returns current linear velocity of the rigid body.
    pub fn lin_vel(&self) -> Vector3<f32> {
        self.lin_vel
    }

    /// Sets new angular velocity of the rigid body. Changing this parameter will wake up the rigid
    /// body!
    pub fn set_ang_vel(&mut self, ang_vel: Vector3<f32>) {
        self.ang_vel = ang_vel;
        self.changes.get_mut().insert(RigidBodyChanges::ANG_VEL);
    }

    /// Returns current angular velocity of the rigid body.
    pub fn ang_vel(&self) -> Vector3<f32> {
        self.ang_vel
    }

    /// Sets _additional_ mass of the rigid body. It is called additional because real mass is defined
    /// by colliders attached to the body and their density and volume.
    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass;
        self.changes.get_mut().insert(RigidBodyChanges::MASS);
    }

    /// Returns _additional_ mass of the rigid body.
    pub fn mass(&self) -> f32 {
        self.mass
    }

    /// Sets angular damping of the rigid body. Angular damping will decrease angular velocity over
    /// time. Default is zero.
    pub fn set_ang_damping(&mut self, damping: f32) {
        self.ang_damping = damping;
        self.changes.get_mut().insert(RigidBodyChanges::ANG_DAMPING);
    }

    /// Returns current angular damping.
    pub fn ang_damping(&self) -> f32 {
        self.ang_damping
    }

    /// Sets linear damping of the rigid body. Linear damping will decrease linear velocity over
    /// time. Default is zero.
    pub fn set_lin_damping(&mut self, damping: f32) {
        self.lin_damping = damping;
        self.changes.get_mut().insert(RigidBodyChanges::LIN_DAMPING);
    }

    /// Returns current linear damping.
    pub fn lin_damping(&self) -> f32 {
        self.lin_damping
    }

    /// Locks rotations around X axis in world coordinates.
    pub fn lock_x_rotations(&mut self, state: bool) {
        self.x_rotation_locked = state;
        self.changes
            .get_mut()
            .insert(RigidBodyChanges::ROTATION_LOCKED);
    }

    /// Returns true if rotation around X axis is locked, false - otherwise.
    pub fn is_x_rotation_locked(&self) -> bool {
        self.x_rotation_locked
    }

    /// Locks rotations around Y axis in world coordinates.
    pub fn lock_y_rotations(&mut self, state: bool) {
        self.y_rotation_locked = state;
        self.changes
            .get_mut()
            .insert(RigidBodyChanges::ROTATION_LOCKED);
    }

    /// Returns true if rotation around Y axis is locked, false - otherwise.    
    pub fn is_y_rotation_locked(&self) -> bool {
        self.y_rotation_locked
    }

    /// Locks rotations around Z axis in world coordinates.
    pub fn lock_z_rotations(&mut self, state: bool) {
        self.z_rotation_locked = state;
        self.changes
            .get_mut()
            .insert(RigidBodyChanges::ROTATION_LOCKED);
    }

    /// Returns true if rotation around Z axis is locked, false - otherwise.    
    pub fn is_z_rotation_locked(&self) -> bool {
        self.z_rotation_locked
    }

    /// Locks or unlocks rotations around all axes at once.
    pub fn lock_rotations(&mut self, locked: bool) {
        self.x_rotation_locked = locked;
        self.y_rotation_locked = locked;
        self.z_rotation_locked = locked;
        self.changes
            .get_mut()
            .insert(RigidBodyChanges::ROTATION_LOCKED);
    }

    /// Locks translation in world coordinates.
    pub fn lock_translation(&mut self, state: bool) {
        self.translation_locked = state;
        self.changes
            .get_mut()
            .insert(RigidBodyChanges::TRANSLATION_LOCKED);
    }

    /// Returns true if translation is locked, false - otherwise.    
    pub fn is_translation_locked(&self) -> bool {
        self.translation_locked
    }

    /// Sets new body type. See [`RigidBodyType`] for more info.
    pub fn set_body_type(&mut self, body_type: RigidBodyType) {
        self.body_type = body_type;
        self.changes.get_mut().insert(RigidBodyChanges::BODY_TYPE);
    }

    /// Returns current body type.
    pub fn body_type(&self) -> RigidBodyType {
        self.body_type
    }

    /// Returns true if the rigid body is sleeping (temporarily excluded from simulation to save
    /// resources), false - otherwise.
    pub fn is_sleeping(&self) -> bool {
        self.sleeping
    }

    /// Returns true if continuous collision detection is enabled, false - otherwise.
    pub fn is_ccd_enabled(&self) -> bool {
        self.ccd_enabled
    }

    /// Enables or disables continuous collision detection. CCD is very useful for fast moving objects
    /// to prevent accidental penetrations on high velocities.
    pub fn enable_ccd(&mut self, enable: bool) {
        self.ccd_enabled = enable;
        self.changes.get_mut().insert(RigidBodyChanges::CCD_STATE);
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
    pub fn set_can_sleep(&mut self, can_sleep: bool) {
        self.can_sleep = can_sleep;
        self.changes.get_mut().insert(RigidBodyChanges::CAN_SLEEP);
    }

    /// Returns true if the rigid body can sleep, false - otherwise.
    pub fn is_can_sleep(&self) -> bool {
        self.can_sleep
    }

    /// Wakes up rigid body, forcing it to return to participate in the simulation.
    pub fn wake_up(&mut self) {
        self.actions.get_mut().push_back(ApplyAction::WakeUp)
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

    /// Creates RigidBody node but does not add it to the graph.
    pub fn build_node(self) -> Node {
        let rigid_body = RigidBody {
            base: self.base_builder.build_base(),
            lin_vel: self.lin_vel,
            ang_vel: self.ang_vel,
            lin_damping: self.lin_damping,
            ang_damping: self.ang_damping,
            sleeping: self.sleeping,
            body_type: self.body_type,
            mass: self.mass,
            x_rotation_locked: self.x_rotation_locked,
            y_rotation_locked: self.y_rotation_locked,
            z_rotation_locked: self.z_rotation_locked,
            translation_locked: self.translation_locked,
            ccd_enabled: self.ccd_enabled,
            can_sleep: self.can_sleep,
            native: Cell::new(RigidBodyHandle::invalid()),
            changes: Cell::new(RigidBodyChanges::NONE),
            actions: Default::default(),
        };

        Node::RigidBody(rigid_body)
    }

    /// Creates RigidBody node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
