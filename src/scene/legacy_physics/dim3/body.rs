//! A container for rigid bodies.

use crate::{
    scene::legacy_physics::dim3::NativeRigidBodyHandle,
    scene::legacy_physics::dim3::RigidBodyHandle,
};
use rapier3d::dynamics::{RigidBody, RigidBodySet};
use rg3d_core::BiDirHashMap;

/// See module docs.
pub struct RigidBodyContainer {
    pub(super) set: RigidBodySet,
    pub(super) handle_map: BiDirHashMap<RigidBodyHandle, NativeRigidBodyHandle>,
}

impl Default for RigidBodyContainer {
    fn default() -> Self {
        Self {
            set: RigidBodySet::new(),
            handle_map: Default::default(),
        }
    }
}

impl RigidBodyContainer {
    /// Creates new rigid body container.
    pub fn new() -> Self {
        Self {
            set: RigidBodySet::new(),
            handle_map: Default::default(),
        }
    }

    /// Tries to create the container from raw parts - the rigid bodies set and handle map.
    pub fn from_raw_parts(
        set: RigidBodySet,
        handle_map: BiDirHashMap<RigidBodyHandle, NativeRigidBodyHandle>,
    ) -> Result<Self, &'static str> {
        assert_eq!(set.len(), handle_map.len());

        for handle in handle_map.forward_map().values() {
            if !set.contains(*handle) {
                return Err(
                    "Unable to create rigid body container because handle map is out of sync!",
                );
            }
        }

        Ok(Self { set, handle_map })
    }

    /// Tries to borrow a rigid body from the container.
    pub fn get_mut(&mut self, handle: &RigidBodyHandle) -> Option<&mut RigidBody> {
        let bodies = &mut self.set;
        self.handle_map
            .value_of(handle)
            .and_then(move |&h| bodies.get_mut(h))
    }

    /// Tries to borrow a rigid body from the container using native Rapier handle.
    pub fn native_mut(&mut self, handle: NativeRigidBodyHandle) -> Option<&mut RigidBody> {
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
    pub fn native_ref(&self, handle: NativeRigidBodyHandle) -> Option<&RigidBody> {
        self.set.get(handle)
    }

    /// Returns a mapping that allows you to map RapierHandle <-> rg3dHandle
    pub fn handle_map(&self) -> &BiDirHashMap<RigidBodyHandle, NativeRigidBodyHandle> {
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

    /// Returns true if the container is empty.
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// Returns a reference to inner set.
    pub fn inner_ref(&self) -> &RigidBodySet {
        &self.set
    }
}
