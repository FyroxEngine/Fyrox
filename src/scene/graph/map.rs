//! An `OriginalHandle -> CopyHandle` map. It is used to map handles to nodes after copying.
//!
//! See [NodeHandleMap] docs for more info.

use crate::{
    core::{pool::Handle, variable::TemplateVariable},
    scene::node::Node,
};
use fxhash::FxHashMap;

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
    pub fn map_slice(&self, handles: &mut [Handle<Node>]) -> &Self {
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
    pub fn try_map_slice(&self, handles: &mut [Handle<Node>]) -> bool {
        let mut success = true;
        for handle in handles {
            success &= self.try_map(handle);
        }
        success
    }

    /// Tries to silently map (without setting `modified` flag) a templated handle to a handle of its origin.
    /// If it exists, the method returns true or false otherwise. It should be used when you not sure that respective
    /// origin exists.
    pub fn try_map_silent(&self, templated_handle: &mut TemplateVariable<Handle<Node>>) -> bool {
        if let Some(new_handle) = self.map.get(templated_handle) {
            templated_handle.set_silent(*new_handle);
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
}
