//! Layer mask is a sort of blacklist that prevents layer from animating certain nodes. See [`LayerMask`] docs
//! for more info.

use crate::{
    core::{pool::Handle, reflect::prelude::*, visitor::prelude::*},
    scene::{graph::Graph, node::Node},
};

/// Layer mask is a sort of blacklist that prevents layer from animating certain nodes. Its main use case is to
/// disable animation on animation layers for specific body parts of humanoid (but not only) characters. The
/// mask holds handles of nodes that **will not** be animated.
#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq, Eq)]
pub struct LayerMask {
    excluded_bones: Vec<Handle<Node>>,
}

impl From<Vec<Handle<Node>>> for LayerMask {
    fn from(mut excluded_bones: Vec<Handle<Node>>) -> Self {
        excluded_bones.sort_by_key(|h| h.index());
        Self { excluded_bones }
    }
}

impl LayerMask {
    /// Creates a layer mask for every descendant node starting from specified `root` (included). It could
    /// be useful if you have an entire node hierarchy (for example, lower part of a body) that needs to
    /// be filtered out.
    pub fn from_hierarchy(graph: &Graph, root: Handle<Node>) -> Self {
        Self::from(graph.traverse_handle_iter(root).collect::<Vec<_>>())
    }

    /// Merges a given layer mask in the current mask, handles will be automatically de-duplicated.
    pub fn merge(&mut self, other: LayerMask) {
        for handle in other.into_inner() {
            if !self.contains(handle) {
                self.add(handle);
            }
        }
    }

    /// Adds a node handle to the mask. You can add as many nodes here as you want and pretty much any handle,
    /// but you should keep handles only to nodes are affected by your animations. Otherwise you'll just make
    /// the inner container bigger and it will degrade in performance.
    ///
    /// # Performance
    ///
    /// The method has O(log(n)) complexity, which means it is very fast for most use cases.
    #[inline]
    pub fn add(&mut self, node: Handle<Node>) {
        let index = self
            .excluded_bones
            .partition_point(|h| h.index() < node.index());

        self.excluded_bones.insert(index, node);
    }

    /// Removes a given node handle from the mask (if any).
    ///
    /// # Performance
    ///
    /// The method has O(log(n)) complexity, which means it is very fast for most use cases.
    pub fn remove(&mut self, node: Handle<Node>) {
        if let Some(index) = self.index_of(node) {
            self.excluded_bones.remove(index);
        }
    }

    fn index_of(&self, node: Handle<Node>) -> Option<usize> {
        if let Ok(mut index) = self
            .excluded_bones
            .binary_search_by(|h| h.index().cmp(&node.index()))
        {
            // We could have multiple handles with the same index, but different generation.
            // In this case we check every handle.
            loop {
                if let Some(current) = self.excluded_bones.get(index) {
                    if current.index() == node.index() {
                        if current.generation() == node.generation() {
                            return Some(index);
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }

                index += 1;
            }
        }

        None
    }

    /// Checks if the mask contains a given node handle or not.
    ///
    /// # Performance
    ///
    /// The method has O(log(n)) complexity, which means it is very fast for most use cases.
    #[inline]
    pub fn contains(&self, node: Handle<Node>) -> bool {
        self.index_of(node).is_some()
    }

    /// Check if a node should be animated or not.
    ///
    /// # Performance
    ///
    /// The method has O(log(n)) complexity, which means it is very fast for most use cases.
    #[inline]
    pub fn should_animate(&self, node: Handle<Node>) -> bool {
        !self.contains(node)
    }

    /// Return a reference to inner container. There's only non-mutable version because inner container must always
    /// be sorted.
    #[inline]
    pub fn inner(&self) -> &Vec<Handle<Node>> {
        &self.excluded_bones
    }

    /// Converts the mask into inner container.
    #[inline]
    pub fn into_inner(self) -> Vec<Handle<Node>> {
        self.excluded_bones
    }
}
