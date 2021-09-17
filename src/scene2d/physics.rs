//! Contains all structures and methods to operate with physics world.

use crate::{
    core::visitor::prelude::*,
    physics2d::{
        body::RigidBodyContainer,
        collider::ColliderContainer,
        desc::PhysicsDesc,
        joint::JointContainer,
        rapier::{
            dynamics::{JointSet, RigidBodySet},
            geometry::ColliderSet,
        },
        PhysicsWorld,
    },
};
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// Physics world.
#[derive(Debug)]
pub struct Physics {
    world: PhysicsWorld,

    /// Descriptors have two purposes:
    /// 1) Defer deserialization to resolve stage - the stage where all meshes
    ///    were loaded and there is a possibility to obtain data for trimeshes.
    ///    Resolve stage will drain these vectors. This is normal use case.
    /// 2) Save data from editor: when descriptors are set, only they will be
    ///    written to output. This is a HACK, but I don't know better solution
    ///    yet.
    pub desc: Option<PhysicsDesc>,
}

impl Visit for Physics {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut desc = if visitor.is_reading() {
            Default::default()
        } else if let Some(desc) = self.desc.as_ref() {
            desc.clone()
        } else {
            self.generate_desc()
        };
        desc.visit("Desc", visitor)?;

        // Save descriptors for resolve stage.
        if visitor.is_reading() {
            self.desc = Some(desc);
        }

        visitor.leave_region()
    }
}

impl Deref for Physics {
    type Target = PhysicsWorld;

    fn deref(&self) -> &Self::Target {
        &self.world
    }
}

impl DerefMut for Physics {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.world
    }
}

impl Default for Physics {
    fn default() -> Self {
        Self {
            world: PhysicsWorld::new(),
            desc: None,
        }
    }
}

impl Physics {
    pub(in crate) fn new() -> Self {
        Self::default()
    }

    // Deep copy is performed using descriptors.
    pub(in crate) fn deep_copy(&self) -> Self {
        let mut phys = Self::new();
        phys.desc = Some(self.generate_desc());
        phys.resolve();
        phys
    }

    pub(in crate) fn resolve(&mut self) {
        assert_eq!(self.bodies.len(), 0);
        assert_eq!(self.colliders.len(), 0);
        assert_eq!(self.joints.len(), 0);

        let mut phys_desc = self.desc.take().unwrap();

        self.integration_parameters = phys_desc.integration_parameters.into();

        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();
        let mut joints = JointSet::new();

        for desc in phys_desc.bodies.drain(..) {
            bodies.insert(desc.convert_to_body());
        }

        for desc in phys_desc.colliders.drain(..) {
            let (collider, parent) = desc.convert_to_collider();
            let parent_handle = phys_desc
                .body_handle_map
                .value_of(&parent)
                .cloned()
                .unwrap();
            colliders.insert_with_parent(collider, parent_handle, &mut bodies);
        }

        for desc in phys_desc.joints.drain(..) {
            let b1 = phys_desc
                .body_handle_map
                .value_of(&desc.body1)
                .cloned()
                .unwrap();
            let b2 = phys_desc
                .body_handle_map
                .value_of(&desc.body2)
                .cloned()
                .unwrap();
            joints.insert(b1, b2, desc.params);
        }

        self.bodies =
            RigidBodyContainer::from_raw_parts(bodies, phys_desc.body_handle_map).unwrap();
        self.colliders =
            ColliderContainer::from_raw_parts(colliders, phys_desc.collider_handle_map).unwrap();
        self.joints = JointContainer::from_raw_parts(joints, phys_desc.joint_handle_map).unwrap();
    }
}
