//! Layer mask is a sort of blacklist that prevents layer from animating certain nodes. See [`LayerMask`] docs
//! for more info.

use crate::core::{reflect::prelude::*, visitor::prelude::*};
use crate::EntityId;

/// Layer mask is a sort of blacklist that prevents layer from animating certain nodes. Its main use case is to
/// disable animation on animation layers for specific body parts of humanoid (but not only) characters. The
/// mask holds handles of nodes that **will not** be animated.
#[derive(Default, Debug, Visit, Reflect, Clone, PartialEq, Eq)]
pub struct LayerMask<T: EntityId> {
    excluded_bones: Vec<T>,
}

impl<T: EntityId> From<Vec<T>> for LayerMask<T> {
    fn from(mut excluded_bones: Vec<T>) -> Self {
        excluded_bones.sort();
        Self { excluded_bones }
    }
}

impl<T: EntityId> LayerMask<T> {
    /// Merges a given layer mask in the current mask, handles will be automatically de-duplicated.
    pub fn merge(&mut self, other: LayerMask<T>) {
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
    pub fn add(&mut self, node: T) {
        let index = self.excluded_bones.partition_point(|h| h < &node);

        self.excluded_bones.insert(index, node);
    }

    /// Removes a given node handle from the mask (if any).
    ///
    /// # Performance
    ///
    /// The method has O(log(n)) complexity, which means it is very fast for most use cases.
    pub fn remove(&mut self, node: T) {
        if let Some(index) = self.index_of(node) {
            self.excluded_bones.remove(index);
        }
    }

    fn index_of(&self, id: T) -> Option<usize> {
        self.excluded_bones.binary_search(&id).ok()
    }

    /// Checks if the mask contains a given node handle or not.
    ///
    /// # Performance
    ///
    /// The method has O(log(n)) complexity, which means it is very fast for most use cases.
    #[inline]
    pub fn contains(&self, node: T) -> bool {
        self.index_of(node).is_some()
    }

    /// Check if a node should be animated or not.
    ///
    /// # Performance
    ///
    /// The method has O(log(n)) complexity, which means it is very fast for most use cases.
    #[inline]
    pub fn should_animate(&self, node: T) -> bool {
        !self.contains(node)
    }

    /// Return a reference to inner container. There's only non-mutable version because inner container must always
    /// be sorted.
    #[inline]
    pub fn inner(&self) -> &Vec<T> {
        &self.excluded_bones
    }

    /// Converts the mask into inner container.
    #[inline]
    pub fn into_inner(self) -> Vec<T> {
        self.excluded_bones
    }
}
