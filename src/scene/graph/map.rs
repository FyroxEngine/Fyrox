//! An `OriginalHandle -> CopyHandle` map. It is used to map handles to nodes after copying.
//!
//! See [NodeHandleMap] docs for more info.

use crate::{
    core::{pool::Handle, reflect::Reflect, variable::InheritableVariable},
    scene::node::Node,
    utils::log::Log,
};
use fxhash::FxHashMap;
use std::ops::{Deref, DerefMut};

/// A `OriginalHandle -> CopyHandle` map. It is used to map handles to nodes after copying.
///
/// Scene nodes have lots of cross references, the simplest cross reference is a handle to parent node,
/// and a set of handles to children nodes. Skinned meshes also store handles to scenes nodes that
/// serve as bones. When you copy a node, you need handles of the copy to point to respective copies.
/// This map allows you to do this.
///
/// Mapping could fail if you do a partial copy of some hierarchy that does not have respective copies of
/// nodes that must be remapped. For example you can copy just a skinned mesh, but not its bones - in this
/// case mapping will fail, but you still can use old handles even it does not make any sense.
#[derive(Default, Clone)]
pub struct NodeHandleMap {
    pub(crate) map: FxHashMap<Handle<Node>, Handle<Node>>,
}

impl NodeHandleMap {
    /// Maps a handle to a handle of its origin, or sets it to [Handle::NONE] if there is no such node.
    /// It should be used when you are sure that respective origin exists.
    pub fn map(&self, handle: &mut Handle<Node>) -> &Self {
        *handle = self.map.get(handle).cloned().unwrap_or_default();
        self
    }

    /// Maps each handle in the slice to a handle of its origin, or sets it to [Handle::NONE] if there is no such node.
    /// It should be used when you are sure that respective origin exists.
    pub fn map_slice<T>(&self, handles: &mut [T]) -> &Self
    where
        T: Deref<Target = Handle<Node>> + DerefMut,
    {
        for handle in handles {
            self.map(handle);
        }
        self
    }

    /// Tries to map a handle to a handle of its origin. If it exists, the method returns true or false otherwise.
    /// It should be used when you not sure that respective origin exists.
    pub fn try_map(&self, handle: &mut Handle<Node>) -> bool {
        if let Some(new_handle) = self.map.get(handle) {
            *handle = *new_handle;
            true
        } else {
            false
        }
    }

    /// Tries to map each handle in the slice to a handle of its origin. If it exists, the method returns true or false otherwise.
    /// It should be used when you not sure that respective origin exists.
    pub fn try_map_slice<T>(&self, handles: &mut [T]) -> bool
    where
        T: Deref<Target = Handle<Node>> + DerefMut,
    {
        let mut success = true;
        for handle in handles {
            success &= self.try_map(handle);
        }
        success
    }

    /// Tries to silently map (without setting `modified` flag) a templated handle to a handle of its origin.
    /// If it exists, the method returns true or false otherwise. It should be used when you not sure that respective
    /// origin exists.
    pub fn try_map_silent(
        &self,
        inheritable_handle: &mut InheritableVariable<Handle<Node>>,
    ) -> bool {
        if let Some(new_handle) = self.map.get(inheritable_handle) {
            inheritable_handle.set_silent(*new_handle);
            true
        } else {
            false
        }
    }

    /// Returns a shared reference to inner map.
    pub fn inner(&self) -> &FxHashMap<Handle<Node>, Handle<Node>> {
        &self.map
    }

    /// Returns inner map.
    pub fn into_inner(self) -> FxHashMap<Handle<Node>, Handle<Node>> {
        self.map
    }

    /// Tries to remap handles to nodes in a given entity using reflection. It finds all supported fields recursively
    /// (`Handle<Node>`, `Vec<Handle<Node>>`, `InheritableVariable<Handle<Node>>`, `InheritableVariable<Vec<Handle<Node>>>`)
    /// and automatically maps old handles to new.
    pub fn remap_handles(&self, node: &mut Node) {
        let name = node.name_owned();
        self.remap_handles_internal(node.as_reflect_mut(), &name);
    }

    fn remap_handles_internal(&self, entity: &mut dyn Reflect, node_name: &str) {
        if let Some(handle) = entity.downcast_mut::<Handle<Node>>() {
            if handle.is_some() && !self.try_map(handle) {
                Log::warn(format!(
                    "Failed to remap handle {} of node {}!",
                    *handle, node_name
                ));
            }
        } else if let Some(vec) = entity.downcast_mut::<Vec<Handle<Node>>>() {
            for handle in vec {
                if handle.is_some() && !self.try_map(handle) {
                    Log::warn(format!(
                        "Failed to remap handle {} in array of node {}!",
                        *handle, node_name
                    ));
                }
            }
        } else if let Some(inheritable) = entity.as_inheritable_variable_mut() {
            // In case of inheritable variable we must take inner value and do not mark variables as modified.
            self.remap_handles_internal(inheritable.inner_value_mut(), node_name);
        } else if let Some(array) = entity.as_array_mut() {
            // Look in every array item.
            for i in 0..array.reflect_len() {
                self.remap_handles_internal(array.reflect_index_mut(i).unwrap(), node_name);
            }
        } else {
            // Continue remapping recursively for every compound field.
            for field in entity.fields_mut() {
                self.remap_handles_internal(field.as_reflect_mut(), node_name);
            }
        }
    }

    pub(crate) fn remap_inheritable_handles(&self, node: &mut Node) {
        let name = node.name_owned();
        self.remap_inheritable_handles_internal(node.as_reflect_mut(), &name, false);
    }

    fn remap_inheritable_handles_internal(
        &self,
        entity: &mut dyn Reflect,
        node_name: &str,
        do_map: bool,
    ) {
        if let Some(handle) = entity.downcast_mut::<Handle<Node>>() {
            if do_map {
                if handle.is_some() && !self.try_map(handle) {
                    Log::warn(format!(
                        "Failed to remap handle {} of node {}!",
                        *handle, node_name
                    ));
                }
            }
        } else if let Some(vec) = entity.downcast_mut::<Vec<Handle<Node>>>() {
            if do_map {
                for handle in vec {
                    if handle.is_some() && !self.try_map(handle) {
                        Log::warn(format!(
                            "Failed to remap handle {} in array of node {}!",
                            *handle, node_name
                        ));
                    }
                }
            }
        } else if let Some(inheritable) = entity.as_inheritable_variable_mut() {
            // In case of inheritable variable we must take inner value and do not mark variables as modified.
            if !inheritable.is_modified() {
                self.remap_inheritable_handles_internal(
                    inheritable.inner_value_mut(),
                    node_name,
                    // Raise mapping flag, any handle in inner value will be mapped. The flag is propagated
                    // to unlimited depth.
                    true,
                );
            }
        } else if let Some(array) = entity.as_array_mut() {
            // Look in every array item.
            for i in 0..array.reflect_len() {
                self.remap_inheritable_handles_internal(
                    array.reflect_index_mut(i).unwrap(),
                    node_name,
                    // Propagate mapping flag - it means that we're inside inheritable variable. In this
                    // case we will map handles.
                    do_map,
                );
            }
        } else {
            // Continue remapping recursively for every compound field.
            for field in entity.fields_mut() {
                self.remap_inheritable_handles_internal(
                    field, node_name,
                    // Propagate mapping flag - it means that we're inside inheritable variable. In this
                    // case we will map handles.
                    do_map,
                );
            }
        }
    }
}
