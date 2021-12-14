#![allow(missing_docs)]

use crate::{
    core::{
        algebra::Vector3,
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::{desc::RigidBodyTypeDesc, rapier::prelude::RigidBodyHandle},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use bitflags::bitflags;
use std::ops::{Deref, DerefMut};

bitflags! {
    pub(crate) struct RigidBodyChanges: u32 {
        const NONE = 0;
        const LIN_VEL = 0b00000001;
        const ANG_VEL = 0b00000010;
        const BODY_TYPE = 0b00000100;
        const ROTATION_LOCKED = 0b00001000;
        const TRANSLATION_LOCKED = 0b00010000;
    }
}

#[derive(Visit, Inspect, Debug)]
pub struct RigidBody {
    base: Base,
    pub lin_vel: Vector3<f32>,
    pub ang_vel: Vector3<f32>,
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
            sleeping: false,
            body_type: Default::default(),
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
            sleeping: self.sleeping,
            body_type: self.body_type,
            mass: 0.0,
            x_rotation_locked: false,
            y_rotation_locked: false,
            z_rotation_locked: false,
            translation_locked: false,
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
}

pub struct RigidBodyBuilder {
    base_builder: BaseBuilder,
    lin_vel: Vector3<f32>,
    ang_vel: Vector3<f32>,
    sleeping: bool,
    status: RigidBodyTypeDesc,
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
            sleeping: false,
            status: Default::default(),
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
            sleeping: self.sleeping,
            body_type: self.status,
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

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
