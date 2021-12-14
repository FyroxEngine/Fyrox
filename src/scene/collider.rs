#![allow(missing_docs)]

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    physics3d::{
        desc::ColliderShapeDesc, desc::InteractionGroupsDesc, rapier::geometry::ColliderHandle,
    },
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use bitflags::bitflags;
use std::ops::{Deref, DerefMut};

bitflags! {
    pub(crate) struct ColliderChanges: u32 {
        const NONE = 0;
        const SHAPE = 0b0000_0001;
        const RESTITUTION = 0b0000_0010;
        const COLLISION_GROUPS = 0b0000_0100;
        const FRICTION = 0b0000_1000;
        const FRICTION_COMBINE_RULE = 0b0001_0000;
        const RESTITUTION_COMBINE_RULE = 0b0010_0000;
        const IS_SENSOR = 0b0100_0000;
        const SOLVER_GROUPS = 0b1000_0000;
        const DENSITY = 0b0001_0000_0000;
    }
}

#[derive(Inspect, Visit, Debug)]
pub struct Collider {
    base: Base,
    #[inspect(read_only)]
    shape: ColliderShapeDesc,
    #[inspect(min_value = 0.0, step = 0.05)]
    friction: f32,
    density: Option<f32>,
    #[inspect(min_value = 0.0, step = 0.05)]
    restitution: f32,
    is_sensor: bool,
    collision_groups: InteractionGroupsDesc,
    solver_groups: InteractionGroupsDesc,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) native: ColliderHandle,
    #[visit(skip)]
    #[inspect(skip)]
    pub(in crate) changes: ColliderChanges,
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
            changes: ColliderChanges::NONE,
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

pub struct ColliderShapeRefMut<'a> {
    parent: &'a mut Collider,
}

impl<'a> Drop for ColliderShapeRefMut<'a> {
    fn drop(&mut self) {
        self.parent.changes.insert(ColliderChanges::SHAPE);
    }
}

impl<'a> Deref for ColliderShapeRefMut<'a> {
    type Target = ColliderShapeDesc;

    fn deref(&self) -> &Self::Target {
        &self.parent.shape
    }
}

impl<'a> DerefMut for ColliderShapeRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.parent.shape
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
            changes: ColliderChanges::NONE,
        }
    }

    pub fn set_shape(&mut self, shape: ColliderShapeDesc) {
        self.shape = shape;
        self.changes.insert(ColliderChanges::SHAPE);
    }

    pub fn shape(&self) -> &ColliderShapeDesc {
        &self.shape
    }

    pub fn shape_mut(&mut self) -> ColliderShapeRefMut {
        ColliderShapeRefMut { parent: self }
    }

    pub fn set_restitution(&mut self, restitution: f32) {
        self.restitution = restitution;
        self.changes.insert(ColliderChanges::RESTITUTION);
    }

    pub fn restitution(&self) -> f32 {
        self.restitution
    }

    pub fn set_density(&mut self, density: Option<f32>) {
        self.density = density;
        self.changes.insert(ColliderChanges::DENSITY);
    }

    pub fn density(&self) -> Option<f32> {
        self.density
    }

    pub fn set_friction(&mut self, friction: f32) {
        self.friction = friction;
        self.changes.insert(ColliderChanges::FRICTION);
    }

    pub fn friction(&self) -> f32 {
        self.friction
    }

    pub fn set_collision_groups(&mut self, groups: InteractionGroupsDesc) {
        self.collision_groups = groups;
        self.changes.insert(ColliderChanges::COLLISION_GROUPS);
    }

    pub fn collision_groups(&self) -> InteractionGroupsDesc {
        self.collision_groups
    }

    pub fn set_solver_groups(&mut self, groups: InteractionGroupsDesc) {
        self.solver_groups = groups;
        self.changes.insert(ColliderChanges::SOLVER_GROUPS);
    }

    pub fn solver_groups(&self) -> InteractionGroupsDesc {
        self.solver_groups
    }

    pub fn set_is_sensor(&mut self, is_sensor: bool) {
        self.is_sensor = is_sensor;
        self.changes.insert(ColliderChanges::IS_SENSOR);
    }

    pub fn is_sensor(&self) -> bool {
        self.is_sensor
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
            changes: ColliderChanges::NONE,
        };
        Node::Collider(collider)
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
