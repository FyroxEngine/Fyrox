//! A container for colliders.

use crate::{body::RigidBodyContainer, ColliderHandle, NativeColliderHandle, RigidBodyHandle};
#[cfg(feature = "dim2")]
use rapier2d::{
    dynamics::IslandManager,
    geometry::{Collider, ColliderSet},
};
#[cfg(feature = "dim3")]
use rapier3d::{
    dynamics::IslandManager,
    geometry::{Collider, ColliderSet},
};
use rg3d_core::{uuid::Uuid, BiDirHashMap};

/// See module docs.
pub struct ColliderContainer {
    pub(super) set: ColliderSet,
    pub(super) handle_map: BiDirHashMap<ColliderHandle, NativeColliderHandle>,
}

impl Default for ColliderContainer {
    fn default() -> Self {
        Self {
            set: ColliderSet::new(),
            handle_map: Default::default(),
        }
    }
}

impl ColliderContainer {
    /// Creates new collider container.
    pub fn new() -> Self {
        Self {
            set: ColliderSet::new(),
            handle_map: Default::default(),
        }
    }

    /// Tries to create the container from raw parts - the collider set and handle map.
    pub fn from_raw_parts(
        set: ColliderSet,
        handle_map: BiDirHashMap<ColliderHandle, NativeColliderHandle>,
    ) -> Result<Self, ()> {
        assert_eq!(set.len(), handle_map.len());

        for handle in handle_map.forward_map().values() {
            if !set.contains(*handle) {
                return Err(());
            }
        }

        Ok(Self { set, handle_map })
    }

    /// Adds new collider to the container.
    pub(super) fn add(
        &mut self,
        collider: Collider,
        parent: &RigidBodyHandle,
        container: &mut RigidBodyContainer,
    ) -> ColliderHandle {
        let handle = self.set.insert_with_parent(
            collider,
            container.handle_map().value_of(parent).cloned().unwrap(),
            &mut container.set,
        );
        let id = ColliderHandle::from(Uuid::new_v4());
        self.handle_map.insert(id, handle);
        id
    }

    /// Removes a collider from the container.
    pub(super) fn remove(
        &mut self,
        collider_handle: &ColliderHandle,
        rigid_body_container: &mut RigidBodyContainer,
        islands: &mut IslandManager,
    ) -> Option<Collider> {
        let colliders = &mut self.set;
        let result = self
            .handle_map
            .value_of(collider_handle)
            .and_then(|&h| colliders.remove(h, islands, &mut rigid_body_container.set, true));
        self.handle_map.remove_by_key(collider_handle);
        result
    }

    /// Tries to borrow a collider from the container.
    pub fn get_mut(&mut self, handle: &ColliderHandle) -> Option<&mut Collider> {
        let colliders = &mut self.set;
        self.handle_map
            .value_of(handle)
            .and_then(move |&h| colliders.get_mut(h))
    }

    /// Tries to borrow a collider from the container using native Rapier handle.
    pub fn native_mut(&mut self, handle: NativeColliderHandle) -> Option<&mut Collider> {
        self.set.get_mut(handle)
    }

    /// Tries to borrow a collider from the container.
    pub fn get(&self, handle: &ColliderHandle) -> Option<&Collider> {
        let colliders = &self.set;
        self.handle_map
            .value_of(handle)
            .and_then(move |&h| colliders.get(h))
    }

    /// Tries to borrow a collider from the container using native Rapier handle.
    pub fn native_ref(&self, handle: NativeColliderHandle) -> Option<&Collider> {
        self.set.get(handle)
    }

    /// Returns a mapping that allows you to map RapierHandle <-> rg3dHandle
    pub fn handle_map(&self) -> &BiDirHashMap<ColliderHandle, NativeColliderHandle> {
        &self.handle_map
    }

    /// Returns true if there is a body with given handle.
    pub fn contains(&self, handle: &ColliderHandle) -> bool {
        self.get(handle).is_some()
    }

    /// Returns an iterator over colliders.
    pub fn iter(&self) -> impl Iterator<Item = &Collider> {
        self.set.iter().map(|(_, b)| b)
    }

    /// Returns an iterator over colliders.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Collider> {
        self.set.iter_mut().map(|(_, b)| b)
    }

    /// Returns an iterator over colliders.
    pub fn pair_iter(&self) -> impl Iterator<Item = (ColliderHandle, &Collider)> {
        self.set
            .iter()
            .map(move |(h, b)| (self.handle_map.key_of(&h).cloned().unwrap(), b))
    }

    /// Returns an iterator over colliders.
    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (ColliderHandle, &mut Collider)> {
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
    pub fn inner_ref(&self) -> &ColliderSet {
        &self.set
    }
}
