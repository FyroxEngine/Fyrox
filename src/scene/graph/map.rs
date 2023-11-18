//! An `OriginalHandle -> CopyHandle` map. It is used to map handles to nodes after copying.
//!
//! See [NodeHandleMap] docs for more info.

use crate::{
    core::{log::Log, pool::Handle, reflect::prelude::*, variable::InheritableVariable},
    scene::node::Node,
};
use fxhash::FxHashMap;
use std::any::TypeId;
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
            inheritable_handle.set_value_silent(*new_handle);
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
    pub fn remap_handles(&self, node: &mut Node, ignored_types: &[TypeId]) {
        let name = node.name_owned();
        node.as_reflect_mut(&mut |node| self.remap_handles_internal(node, &name, ignored_types));
    }

    fn remap_handles_internal(
        &self,
        entity: &mut dyn Reflect,
        node_name: &str,
        ignored_types: &[TypeId],
    ) {
        if ignored_types.contains(&(*entity).type_id()) {
            return;
        }

        let mut mapped = false;

        entity.downcast_mut::<Handle<Node>>(&mut |handle| {
            if let Some(handle) = handle {
                if handle.is_some() && !self.try_map(handle) {
                    Log::warn(format!(
                        "Failed to remap handle {} of node {}!",
                        *handle, node_name
                    ));
                }
                mapped = true;
            }
        });

        if mapped {
            return;
        }

        entity.downcast_mut::<Vec<Handle<Node>>>(&mut |vec| {
            if let Some(vec) = vec {
                for handle in vec {
                    if handle.is_some() && !self.try_map(handle) {
                        Log::warn(format!(
                            "Failed to remap handle {} in array of node {}!",
                            *handle, node_name
                        ));
                    }
                }
                mapped = true;
            }
        });

        if mapped {
            return;
        }

        entity.as_inheritable_variable_mut(&mut |inheritable| {
            if let Some(inheritable) = inheritable {
                // In case of inheritable variable we must take inner value and do not mark variables as modified.
                self.remap_handles_internal(
                    inheritable.inner_value_mut(),
                    node_name,
                    ignored_types,
                );

                mapped = true;
            }
        });

        if mapped {
            return;
        }

        entity.as_array_mut(&mut |array| {
            if let Some(array) = array {
                // Look in every array item.
                for i in 0..array.reflect_len() {
                    // Sparse arrays (like Pool) could have empty entries.
                    if let Some(item) = array.reflect_index_mut(i) {
                        self.remap_handles_internal(item, node_name, ignored_types);
                    }
                }
                mapped = true;
            }
        });

        if mapped {
            return;
        }

        // Continue remapping recursively for every compound field.
        entity.fields_mut(&mut |fields| {
            for field in fields {
                field.as_reflect_mut(&mut |field| {
                    self.remap_handles_internal(field, node_name, ignored_types)
                })
            }
        })
    }

    pub(crate) fn remap_inheritable_handles(&self, node: &mut Node, ignored_types: &[TypeId]) {
        let name = node.name_owned();
        node.as_reflect_mut(&mut |node| {
            self.remap_inheritable_handles_internal(node, &name, false, ignored_types)
        });
    }

    fn remap_inheritable_handles_internal(
        &self,
        entity: &mut dyn Reflect,
        node_name: &str,
        do_map: bool,
        ignored_types: &[TypeId],
    ) {
        if ignored_types.contains(&(*entity).type_id()) {
            return;
        }

        let mut mapped = false;

        entity.as_inheritable_variable_mut(&mut |result| {
            if let Some(inheritable) = result {
                // In case of inheritable variable we must take inner value and do not mark variables as modified.
                if !inheritable.is_modified() {
                    self.remap_inheritable_handles_internal(
                        inheritable.inner_value_mut(),
                        node_name,
                        // Raise mapping flag, any handle in inner value will be mapped. The flag is propagated
                        // to unlimited depth.
                        true,
                        ignored_types,
                    );
                }
                mapped = true;
            }
        });

        if mapped {
            return;
        }

        entity.downcast_mut::<Handle<Node>>(&mut |result| {
            if let Some(handle) = result {
                if do_map && handle.is_some() && !self.try_map(handle) {
                    Log::warn(format!(
                        "Failed to remap handle {} of node {}!",
                        *handle, node_name
                    ));
                }
                mapped = true;
            }
        });

        if mapped {
            return;
        }

        entity.downcast_mut::<Vec<Handle<Node>>>(&mut |result| {
            if let Some(vec) = result {
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
                mapped = true;
            }
        });

        if mapped {
            return;
        }

        entity.as_array_mut(&mut |result| {
            if let Some(array) = result {
                // Look in every array item.
                for i in 0..array.reflect_len() {
                    // Sparse arrays (like Pool) could have empty entries.
                    if let Some(item) = array.reflect_index_mut(i) {
                        self.remap_inheritable_handles_internal(
                            item,
                            node_name,
                            // Propagate mapping flag - it means that we're inside inheritable variable. In this
                            // case we will map handles.
                            do_map,
                            ignored_types,
                        );
                    }
                }
                mapped = true;
            }
        });

        if mapped {
            return;
        }

        // Continue remapping recursively for every compound field.
        entity.fields_mut(&mut |fields| {
            for field in fields {
                self.remap_inheritable_handles_internal(
                    field,
                    node_name,
                    // Propagate mapping flag - it means that we're inside inheritable variable. In this
                    // case we will map handles.
                    do_map,
                    ignored_types,
                );
            }
        })
    }
}
