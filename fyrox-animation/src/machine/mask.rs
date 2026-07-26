// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Layer mask is a sort of blacklist that prevents layer from animating certain nodes. See [`LayerMask`] docs
//! for more info.

use crate::core::{reflect::prelude::*, visitor::prelude::*};
use crate::EntityId;

/// Layer mask is a sort of blacklist that prevents layer from animating certain nodes. Its main use case is to
/// disable animation on animation layers for specific body parts of humanoid (but not only) characters. The
/// mask holds handles of nodes that **will not** be animated.
#[derive(Default, Debug, Visit, Clone, PartialEq, Eq)]
pub struct LayerMask<T: EntityId> {
    excluded_bones: Vec<T>,
    #[visit(skip)]
    sorted: bool,
}

static EXCLUDED_BONES_METADATA: FieldMetadata = FieldMetadata {
    name: "ExcludedBones",
    display_name: "ExcludedBones",
    tag: "",
    read_only: false,
    immutable_collection: false,
    min_value: None,
    max_value: None,
    step: None,
    precision: None,
    doc: "",
};

impl<T: EntityId> Reflect for LayerMask<T> {
    fn type_info() -> TypeInfo
    where
        Self: Sized,
    {
        TypeInfo {
            source_path: file!(),
            type_name: std::any::type_name::<Self>(),
            assembly_name: env!("CARGO_PKG_NAME"),
            doc_comment: "",
            derived_types: &[],
            type_uuid: uuid!("fde99dd1-444f-4c38-a4be-7716933c3115"),
        }
    }

    fn type_info_ref(&self) -> TypeInfo {
        Self::type_info()
    }

    fn try_clone_box(&self) -> Option<Box<dyn Reflect>> {
        Some(Box::new(self.clone()))
    }

    fn try_compare(&self, other: &dyn Reflect) -> Option<bool> {
        (other as &dyn std::any::Any)
            .downcast_ref::<Self>()
            .map(|other| other == self)
    }

    fn fields_ref(&self, func: &mut dyn FnMut(&[FieldRef])) {
        func(&[{
            FieldRef {
                metadata: &EXCLUDED_BONES_METADATA,
                value: &self.excluded_bones,
            }
        }])
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [FieldMut])) {
        self.sorted = false;
        func(&mut [{
            FieldMut {
                metadata: &EXCLUDED_BONES_METADATA,
                value: &mut self.excluded_bones,
            }
        }])
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        let this = std::mem::replace(self, value.take()?);
        Ok(Box::new(this))
    }

    fn field_direct_ref(&self, index: usize) -> Option<FieldRef> {
        if index == 0 {
            Some(FieldRef {
                metadata: &EXCLUDED_BONES_METADATA,
                value: &self.excluded_bones,
            })
        } else {
            None
        }
    }

    fn field_direct_mut(&mut self, index: usize) -> Option<FieldMut> {
        if index == 0 {
            self.sorted = false;
            Some(FieldMut {
                metadata: &EXCLUDED_BONES_METADATA,
                value: &mut self.excluded_bones,
            })
        } else {
            None
        }
    }
}

impl<T: EntityId> From<Vec<T>> for LayerMask<T> {
    fn from(mut excluded_bones: Vec<T>) -> Self {
        excluded_bones.sort();
        Self {
            excluded_bones,
            sorted: true,
        }
    }
}

impl<T: EntityId> LayerMask<T> {
    fn ensure_sorted(&mut self) {
        self.excluded_bones.sort();
        self.sorted = true;
    }

    /// Merges a given layer mask in the current mask, handles will be automatically de-duplicated.
    pub fn merge(&mut self, other: LayerMask<T>) {
        self.ensure_sorted();
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
        self.ensure_sorted();

        let index = self.excluded_bones.partition_point(|h| h < &node);

        self.excluded_bones.insert(index, node);
    }

    /// Removes a given node handle from the mask (if any).
    ///
    /// # Performance
    ///
    /// The method has O(log(n)) complexity, which means it is very fast for most use cases.
    pub fn remove(&mut self, node: T) {
        self.ensure_sorted();

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
    pub fn contains(&mut self, node: T) -> bool {
        self.ensure_sorted();
        self.index_of(node).is_some()
    }

    /// Check if a node should be animated or not.
    ///
    /// # Performance
    ///
    /// The method has O(log(n)) complexity, which means it is very fast for most use cases.
    #[inline]
    pub fn should_animate(&mut self, node: T) -> bool {
        self.ensure_sorted();
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
