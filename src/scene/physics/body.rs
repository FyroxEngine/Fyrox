//! A container for rigid bodies.

use crate::{
    core::{uuid::Uuid, BiDirHashMap},
    engine::RigidBodyHandle,
    physics::dynamics::RigidBody,
    scene::physics::{collider::ColliderContainer, joint::JointContainer},
};
use rapier3d::dynamics::{IslandManager, RigidBodySet};

/// See module docs.
pub struct RigidBodyContainer {
    pub(super) set: RigidBodySet,
    pub(super) handle_map: BiDirHashMap<RigidBodyHandle, rapier3d::dynamics::RigidBodyHandle>,
}

impl RigidBodyContainer {
    /// Creates new rigid body container.
    pub fn new() -> Self {
        Self {
            set: RigidBodySet::new(),
            handle_map: Default::default(),
        }
    }

    /// Adds new rigid body to the container.
    pub(super) fn add(&mut self, rigid_body: RigidBody) -> RigidBodyHandle {
        let handle = self.set.insert(rigid_body);
        let id = RigidBodyHandle::from(Uuid::new_v4());
        self.handle_map.insert(id, handle);
        id
    }

    /// Removes a rigid body from the container.
    pub(super) fn remove(
        &mut self,
        rigid_body: &RigidBodyHandle,
        colliders: &mut ColliderContainer,
        joints: &mut JointContainer,
        islands: &mut IslandManager,
    ) -> Option<RigidBody> {
        let bodies = &mut self.set;
        let result = self
            .handle_map
            .value_of(rigid_body)
            .and_then(|&h| bodies.remove(h, islands, &mut colliders.set, &mut joints.set));
        if let Some(body) = result.as_ref() {
            for collider in body.colliders() {
                colliders.handle_map.remove_by_value(collider);
            }
            self.handle_map.remove_by_key(rigid_body);
        }
        result
    }

    /// Tries to borrow a rigid body from the container.
    pub fn get_mut(&mut self, handle: &RigidBodyHandle) -> Option<&mut RigidBody> {
        let bodies = &mut self.set;
        self.handle_map
            .value_of(handle)
            .and_then(move |&h| bodies.get_mut(h))
    }

    /// Tries to borrow a rigid body from the container using native Rapier handle.
    pub fn native_mut(
        &mut self,
        handle: rapier3d::dynamics::RigidBodyHandle,
    ) -> Option<&mut RigidBody> {
        self.set.get_mut(handle)
    }

    /// Tries to borrow a rigid body from the container.
    pub fn get(&self, handle: &RigidBodyHandle) -> Option<&RigidBody> {
        let bodies = &self.set;
        self.handle_map
            .value_of(handle)
            .and_then(move |&h| bodies.get(h))
    }

    /// Tries to borrow a rigid body from the container using native Rapier handle.
    pub fn native_ref(&self, handle: rapier3d::dynamics::RigidBodyHandle) -> Option<&RigidBody> {
        self.set.get(handle)
    }

    /// Returns a mapping that allows you to map RapierHandle <-> rg3dHandle
    pub fn handle_map(
        &self,
    ) -> &BiDirHashMap<RigidBodyHandle, rapier3d::dynamics::RigidBodyHandle> {
        &self.handle_map
    }

    /// Returns true if there is a body with given handle.
    pub fn contains(&self, handle: &RigidBodyHandle) -> bool {
        self.get(handle).is_some()
    }

    /// Returns an iterator over rigid bodies.
    pub fn iter(&self) -> impl Iterator<Item = &RigidBody> {
        self.set.iter().map(|(_, b)| b)
    }

    /// Returns an iterator over rigid bodies.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RigidBody> {
        self.set.iter_mut().map(|(_, b)| b)
    }

    /// Returns an iterator over rigid bodies.
    pub fn pair_iter(&self) -> impl Iterator<Item = (RigidBodyHandle, &RigidBody)> {
        self.set
            .iter()
            .map(move |(h, b)| (self.handle_map.key_of(&h).cloned().unwrap(), b))
    }

    /// Returns an iterator over rigid bodies.
    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (RigidBodyHandle, &mut RigidBody)> {
        let handle_map = &self.handle_map;
        self.set
            .iter_mut()
            .map(move |(h, b)| (handle_map.key_of(&h).cloned().unwrap(), b))
    }

    /// Returns a length of the container.
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// Returns a reference to inner set.
    pub fn inner_ref(&self) -> &RigidBodySet {
        &self.set
    }
}
