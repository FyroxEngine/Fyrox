//! A container for joints.

use crate::{body::RigidBodyContainer, JointHandle, NativeJointHandle, RigidBodyHandle};
#[cfg(feature = "dim2")]
use rapier2d::dynamics::{IslandManager, Joint, JointParams, JointSet};
#[cfg(feature = "dim3")]
use rapier3d::dynamics::{IslandManager, Joint, JointParams, JointSet};
use rg3d_core::{uuid::Uuid, BiDirHashMap};

/// See module docs.
pub struct JointContainer {
    pub(super) set: JointSet,
    pub(super) handle_map: BiDirHashMap<JointHandle, NativeJointHandle>,
}

impl Default for JointContainer {
    fn default() -> Self {
        Self {
            set: JointSet::new(),
            handle_map: Default::default(),
        }
    }
}

impl JointContainer {
    /// Creates new joint container.
    pub fn new() -> Self {
        Self {
            set: JointSet::new(),
            handle_map: Default::default(),
        }
    }

    /// Tries to create the container from raw parts - the joint set and handle map.
    pub fn from_raw_parts(
        set: JointSet,
        handle_map: BiDirHashMap<JointHandle, NativeJointHandle>,
    ) -> Result<Self, &'static str> {
        assert_eq!(set.len(), handle_map.len());

        for handle in handle_map.forward_map().values() {
            if !set.contains(*handle) {
                return Err("Unable to create joint container because handle map is out of sync!");
            }
        }

        Ok(Self { set, handle_map })
    }

    /// Adds new joint.
    pub(super) fn add<J>(
        &mut self,
        body1: &RigidBodyHandle,
        body2: &RigidBodyHandle,
        joint_params: J,
        bodies: &mut RigidBodyContainer,
    ) -> JointHandle
    where
        J: Into<JointParams>,
    {
        let body1 = *bodies.handle_map().value_of(body1).unwrap();
        let body2 = *bodies.handle_map().value_of(body2).unwrap();

        let handle = self.set.insert(body1, body2, joint_params);
        let id = JointHandle::from(Uuid::new_v4());
        self.handle_map.insert(id, handle);
        id
    }

    /// Removes a joint.
    pub(super) fn remove(
        &mut self,
        joint_handle: &JointHandle,
        bodies: &mut RigidBodyContainer,
        islands: &mut IslandManager,
        wake_up: bool,
    ) -> Option<Joint> {
        let joints = &mut self.set;
        let result = self
            .handle_map
            .value_of(joint_handle)
            .and_then(|&h| joints.remove(h, islands, &mut bodies.set, wake_up));
        self.handle_map.remove_by_key(joint_handle);
        result
    }

    /// Tries to borrow a joint from the container.
    pub fn get_mut(&mut self, handle: &JointHandle) -> Option<&mut Joint> {
        let joints = &mut self.set;
        self.handle_map
            .value_of(handle)
            .and_then(move |&h| joints.get_mut(h))
    }

    /// Tries to borrow a joint from the container using native Rapier handle.
    pub fn native_mut(&mut self, handle: NativeJointHandle) -> Option<&mut Joint> {
        self.set.get_mut(handle)
    }

    /// Tries to borrow a joint from the container.
    pub fn get(&self, handle: &JointHandle) -> Option<&Joint> {
        let bodies = &self.set;
        self.handle_map
            .value_of(handle)
            .and_then(move |&h| bodies.get(h))
    }

    /// Tries to borrow a joint from the container using native Rapier handle.
    pub fn native_ref(&self, handle: NativeJointHandle) -> Option<&Joint> {
        self.set.get(handle)
    }

    /// Returns a mapping that allows you to map RapierHandle <-> rg3dHandle
    pub fn handle_map(&self) -> &BiDirHashMap<JointHandle, NativeJointHandle> {
        &self.handle_map
    }

    /// Returns true if there is a joint with given handle.
    pub fn contains(&self, handle: &JointHandle) -> bool {
        self.get(handle).is_some()
    }

    /// Returns an iterator over joints.
    pub fn iter(&self) -> impl Iterator<Item = &Joint> {
        self.set.iter().map(|(_, b)| b)
    }

    /// Returns an iterator over joints.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Joint> {
        self.set.iter_mut().map(|(_, b)| b)
    }

    /// Returns an iterator over joints.
    pub fn pair_iter(&self) -> impl Iterator<Item = (JointHandle, &Joint)> {
        self.set
            .iter()
            .map(move |(h, b)| (self.handle_map.key_of(&h).cloned().unwrap(), b))
    }

    /// Returns an iterator over joints.
    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (JointHandle, &mut Joint)> {
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
    pub fn inner_ref(&self) -> &JointSet {
        &self.set
    }
}
