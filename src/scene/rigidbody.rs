#![allow(missing_docs)]

use crate::core::algebra::Vector3;
use crate::core::inspect::{Inspect, PropertyInfo};
use crate::core::visitor::prelude::*;
use crate::physics3d::desc::RigidBodyTypeDesc;
use crate::physics3d::rapier::prelude::RigidBodyHandle;
use crate::scene::base::Base;
use std::ops::{Deref, DerefMut};

#[derive(Visit, Inspect, Debug)]
pub struct RigidBody {
    base: Base,
    pub lin_vel: Vector3<f32>,
    pub ang_vel: Vector3<f32>,
    #[inspect(read_only)]
    pub sleeping: bool,
    pub status: RigidBodyTypeDesc,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub mass: f32,
    pub x_rotation_locked: bool,
    pub y_rotation_locked: bool,
    pub z_rotation_locked: bool,
    pub translation_locked: bool,
    #[visit(skip)]
    #[inspect(skip)]
    pub(crate) native: RigidBodyHandle,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            base: Default::default(),
            lin_vel: Default::default(),
            ang_vel: Default::default(),
            sleeping: false,
            status: Default::default(),
            mass: 1.0,
            x_rotation_locked: false,
            y_rotation_locked: false,
            z_rotation_locked: false,
            translation_locked: false,
            native: RigidBodyHandle::invalid(),
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
            status: self.status,
            mass: 0.0,
            x_rotation_locked: false,
            y_rotation_locked: false,
            z_rotation_locked: false,
            translation_locked: false,
            // Do not copy.
            native: RigidBodyHandle::invalid(),
        }
    }
}
