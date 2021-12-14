#![allow(missing_docs)]

use crate::core::inspect::{Inspect, PropertyInfo};
use crate::core::visitor::prelude::*;
use crate::physics3d::desc::ColliderShapeDesc;
use crate::physics3d::desc::InteractionGroupsDesc;
use crate::physics3d::rapier::geometry::ColliderHandle;
use crate::scene::base::{Base, BaseBuilder};
use crate::scene::graph::Graph;
use crate::scene::node::Node;
use rg3d_core::pool::Handle;
use std::ops::{Deref, DerefMut};

#[derive(Inspect, Visit, Debug)]
pub struct Collider {
    base: Base,
    #[inspect(read_only)]
    pub(in crate) shape: ColliderShapeDesc,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub(in crate) friction: f32,
    pub(in crate) density: Option<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    pub(in crate) restitution: f32,
    pub(in crate) is_sensor: bool,
    pub(in crate) collision_groups: InteractionGroupsDesc,
    pub(in crate) solver_groups: InteractionGroupsDesc,
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

pub struct ColliderBuilder {
    base_builder: BaseBuilder,
    shape: ColliderShapeDesc,
    friction: f32,
    density: Option<f32>,
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroupsDesc,
    solver_groups: InteractionGroupsDesc,
}

impl ColliderBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            shape: Default::default(),
            friction: 0.0,
            density: None,
            restitution: 0.0,
            is_sensor: false,
            collision_groups: Default::default(),
            solver_groups: Default::default(),
        }
    }

    pub fn with_shape(mut self, shape: ColliderShapeDesc) -> Self {
        self.shape = shape;
        self
    }

    pub fn build_node(self) -> Node {
        let collider = Collider {
            base: self.base_builder.build_base(),
            shape: self.shape,
            friction: self.friction,
            density: self.density,
            restitution: self.restitution,
            is_sensor: self.is_sensor,
            collision_groups: self.collision_groups,
            solver_groups: self.solver_groups,
            native: ColliderHandle::invalid(),
        };
        Node::Collider(collider)
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
