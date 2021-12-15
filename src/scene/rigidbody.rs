#![allow(missing_docs)]

use crate::{
    core::{
        algebra::Vector3,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::rapier::{dynamics::RigidBodyType, prelude::RigidBodyHandle},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use bitflags::bitflags;
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone, Debug, Inspect, Visit)]
#[repr(u32)]
pub enum RigidBodyTypeDesc {
    Dynamic = 0,
    Static = 1,
    KinematicPositionBased = 2,
    KinematicVelocityBased = 3,
}

impl Default for RigidBodyTypeDesc {
    fn default() -> Self {
        Self::Dynamic
    }
}

impl From<RigidBodyType> for RigidBodyTypeDesc {
    fn from(s: RigidBodyType) -> Self {
        match s {
            RigidBodyType::Dynamic => Self::Dynamic,
            RigidBodyType::Static => Self::Static,
            RigidBodyType::KinematicPositionBased => Self::KinematicPositionBased,
            RigidBodyType::KinematicVelocityBased => Self::KinematicVelocityBased,
        }
    }
}

impl From<RigidBodyTypeDesc> for RigidBodyType {
    fn from(v: RigidBodyTypeDesc) -> Self {
        match v {
            RigidBodyTypeDesc::Dynamic => RigidBodyType::Dynamic,
            RigidBodyTypeDesc::Static => RigidBodyType::Static,
            RigidBodyTypeDesc::KinematicPositionBased => RigidBodyType::KinematicPositionBased,
            RigidBodyTypeDesc::KinematicVelocityBased => RigidBodyType::KinematicVelocityBased,
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
    }
}

#[derive(Visit, Inspect, Debug)]
pub struct RigidBody {
    base: Base,
    pub lin_vel: Vector3<f32>,
    pub ang_vel: Vector3<f32>,
    pub lin_damping: f32,
    pub ang_damping: f32,
    #[inspect(read_only)]
    pub sleeping: bool,
    pub body_type: RigidBodyTypeDesc,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub mass: f32,
    pub x_rotation_locked: bool,
    pub y_rotation_locked: bool,
    pub z_rotation_locked: bool,
    pub translation_locked: bool,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) native: RigidBodyHandle,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) changes: RigidBodyChanges,
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
            body_type: RigidBodyTypeDesc::Dynamic,
            mass: 1.0,
            x_rotation_locked: false,
            y_rotation_locked: false,
            z_rotation_locked: false,
            translation_locked: false,
            native: RigidBodyHandle::invalid(),
            changes: RigidBodyChanges::NONE,
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
            // Do not copy.
            native: RigidBodyHandle::invalid(),
            changes: RigidBodyChanges::NONE,
        }
    }

    pub fn set_lin_vel(&mut self, lin_vel: Vector3<f32>) {
        self.lin_vel = lin_vel;
        self.changes.insert(RigidBodyChanges::LIN_VEL);
    }

    pub fn lin_vel(&self) -> Vector3<f32> {
        self.lin_vel
    }

    pub fn set_ang_vel(&mut self, ang_vel: Vector3<f32>) {
        self.ang_vel = ang_vel;
        self.changes.insert(RigidBodyChanges::ANG_VEL);
    }

    pub fn ang_vel(&self) -> Vector3<f32> {
        self.ang_vel
    }

    pub fn set_mass(&mut self, mass: f32) {
        self.mass = mass;
        self.changes.insert(RigidBodyChanges::MASS);
    }

    pub fn mass(&self) -> f32 {
        self.mass
    }

    pub fn set_ang_damping(&mut self, damping: f32) {
        self.ang_damping = damping;
        self.changes.insert(RigidBodyChanges::ANG_DAMPING);
    }

    pub fn ang_damping(&self) -> f32 {
        self.ang_damping
    }

    pub fn set_lin_damping(&mut self, damping: f32) {
        self.lin_damping = damping;
        self.changes.insert(RigidBodyChanges::LIN_DAMPING);
    }

    pub fn lin_damping(&self) -> f32 {
        self.lin_damping
    }

    pub fn lock_x_rotations(&mut self, state: bool) {
        self.x_rotation_locked = state;
        self.changes.insert(RigidBodyChanges::ROTATION_LOCKED);
    }

    pub fn is_x_rotation_locked(&self) -> bool {
        self.x_rotation_locked
    }

    pub fn lock_y_rotations(&mut self, state: bool) {
        self.y_rotation_locked = state;
        self.changes.insert(RigidBodyChanges::ROTATION_LOCKED);
    }

    pub fn is_y_rotation_locked(&self) -> bool {
        self.y_rotation_locked
    }

    pub fn lock_z_rotations(&mut self, state: bool) {
        self.z_rotation_locked = state;
        self.changes.insert(RigidBodyChanges::ROTATION_LOCKED);
    }

    pub fn is_z_rotation_locked(&self) -> bool {
        self.z_rotation_locked
    }

    pub fn lock_translation(&mut self, state: bool) {
        self.translation_locked = state;
        self.changes.insert(RigidBodyChanges::TRANSLATION_LOCKED);
    }

    pub fn is_translation_locked(&self) -> bool {
        self.translation_locked
    }

    pub fn set_body_type(&mut self, body_type: RigidBodyTypeDesc) {
        self.body_type = body_type;
        self.changes.insert(RigidBodyChanges::BODY_TYPE);
    }

    pub fn body_type(&self) -> RigidBodyTypeDesc {
        self.body_type
    }
}

pub struct RigidBodyBuilder {
    base_builder: BaseBuilder,
    lin_vel: Vector3<f32>,
    ang_vel: Vector3<f32>,
    lin_damping: f32,
    ang_damping: f32,
    sleeping: bool,
    body_type: RigidBodyTypeDesc,
    mass: f32,
    x_rotation_locked: bool,
    y_rotation_locked: bool,
    z_rotation_locked: bool,
    translation_locked: bool,
}

impl RigidBodyBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            lin_vel: Default::default(),
            ang_vel: Default::default(),
            lin_damping: 0.0,
            ang_damping: 0.0,
            sleeping: false,
            body_type: RigidBodyTypeDesc::Dynamic,
            mass: 1.0,
            x_rotation_locked: false,
            y_rotation_locked: false,
            z_rotation_locked: false,
            translation_locked: false,
        }
    }

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
            native: RigidBodyHandle::invalid(),
            changes: RigidBodyChanges::NONE,
        };

        Node::RigidBody(rigid_body)
    }

    pub fn with_body_type(mut self, body_type: RigidBodyTypeDesc) -> Self {
        self.body_type = body_type;
        self
    }

    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    pub fn with_lin_vel(mut self, lin_vel: Vector3<f32>) -> Self {
        self.lin_vel = lin_vel;
        self
    }

    pub fn with_ang_vel(mut self, ang_vel: Vector3<f32>) -> Self {
        self.ang_vel = ang_vel;
        self
    }

    pub fn with_ang_damping(mut self, ang_damping: f32) -> Self {
        self.ang_damping = ang_damping;
        self
    }

    pub fn with_lin_damping(mut self, lin_damping: f32) -> Self {
        self.lin_damping = lin_damping;
        self
    }

    pub fn with_x_rotation_locked(mut self, x_rotation_locked: bool) -> Self {
        self.x_rotation_locked = x_rotation_locked;
        self
    }

    pub fn with_y_rotation_locked(mut self, y_rotation_locked: bool) -> Self {
        self.y_rotation_locked = y_rotation_locked;
        self
    }

    pub fn with_z_rotation_locked(mut self, z_rotation_locked: bool) -> Self {
        self.z_rotation_locked = z_rotation_locked;
        self
    }

    pub fn with_translation_locked(mut self, translation_locked: bool) -> Self {
        self.translation_locked = translation_locked;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
