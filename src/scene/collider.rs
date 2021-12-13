#![allow(missing_docs)]

use crate::core::inspect::{Inspect, PropertyInfo};
use crate::core::visitor::prelude::*;
use crate::physics3d::desc::ColliderShapeDesc;
use crate::physics3d::desc::InteractionGroupsDesc;
use crate::physics3d::rapier::geometry::ColliderHandle;
use crate::scene::base::Base;
use std::ops::{Deref, DerefMut};

#[derive(Inspect, Visit, Debug)]
pub struct Collider {
    base: Base,
    #[inspect(read_only)]
    pub shape: ColliderShapeDesc,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub friction: f32,
    pub density: Option<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub restitution: f32,
    pub is_sensor: bool,
    pub collision_groups: InteractionGroupsDesc,
    pub solver_groups: InteractionGroupsDesc,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) native: ColliderHandle,
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            base: Default::default(),
            shape: Default::default(),
            friction: 0.0,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            collision_groups: Default::default(),
            solver_groups: Default::default(),
            native: ColliderHandle::invalid(),
        }
    }
}

impl Deref for Collider {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Collider {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Collider {
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            shape: self.shape.clone(),
            friction: self.friction,
            density: self.density,
            restitution: self.restitution,
            is_sensor: self.is_sensor,
            collision_groups: self.collision_groups,
            solver_groups: self.solver_groups,
            // Do not copy.
            native: ColliderHandle::invalid(),
        }
    }
}
