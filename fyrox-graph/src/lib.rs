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

#![allow(clippy::type_complexity)]

//! Graph utilities and common algorithms.

pub mod constructor;

pub mod prelude {
    pub use crate::SceneGraph;
}

use fxhash::FxHashMap;
use fyrox_core::pool::{ObjectOrVariant, PoolError};
use fyrox_core::reflect::ReflectHandle;
use fyrox_core::uuid::Uuid;
use fyrox_core::{
    log::{Log, MessageKind},
    pool::Handle,
    reflect::prelude::*,
    variable::{self, InheritableVariable},
    NameProvider,
};
use fyrox_resource::{untyped::UntypedResource, Resource, TypedResourceData};
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Reflect)]
#[reflect(type_uuid = "057d3a49-913a-4f83-9203-549a5c2872aa")]
#[repr(u32)]
pub enum NodeMapping {
    UseNames = 0,
    UseHandles = 1,
}

/// A `OriginalHandle -> CopyHandle` map. It is used to map handles to nodes after copying.
///
/// For example, scene nodes have lots of cross references, the simplest cross reference is a handle
/// to parent node, and a set of handles to children nodes. Skinned meshes also store handles to
/// scenes nodes that serve as bones. When you copy a node, you need handles of the copy to point
/// to respective copies. This map allows you to do this.
///
/// Mapping could fail if you do a partial copy of some hierarchy that does not have respective
/// copies of nodes that must be remapped. For example you can copy just a skinned mesh, but not
/// its bones - in this case mapping will fail, but you still can use old handles even it does not
/// make any sense.
pub struct NodeHandleMap<N> {
    pub(crate) map: FxHashMap<Handle<N>, Handle<N>>,
}

impl<N> Debug for NodeHandleMap<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (from, to) in self.map.iter() {
            writeln!(f, "{from} -> {to}")?;
        }
        Ok(())
    }
}

impl<N> Default for NodeHandleMap<N> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<N> Clone for NodeHandleMap<N> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}

impl<N> NodeHandleMap<N>
where
    N: Reflect + NameProvider,
{
    /// Adds new `original -> copy` handle mapping.
    #[inline]
    pub fn insert(
        &mut self,
        original_handle: Handle<N>,
        copy_handle: Handle<N>,
    ) -> Option<Handle<N>> {
        self.map.insert(original_handle, copy_handle)
    }

    /// Maps a handle to a handle of its origin, or sets it to [Handle::NONE] if there is no such node.
    /// It should be used when you are sure that respective origin exists.
    #[inline]
    pub fn map(&self, handle: &mut Handle<N>) -> &Self {
        *handle = self.map.get(handle).cloned().unwrap_or_default();
        self
    }

    /// Maps each handle in the slice to a handle of its origin, or sets it to [Handle::NONE] if there is no such node.
    /// It should be used when you are sure that respective origin exists.
    #[inline]
    pub fn map_slice<T>(&self, handles: &mut [T]) -> &Self
    where
        T: Deref<Target = Handle<N>> + DerefMut,
    {
        for handle in handles {
            self.map(handle);
        }
        self
    }

    /// Tries to map a handle to a handle of its origin. If it exists, the method returns true or false otherwise.
    /// It should be used when you not sure that respective origin exists.
    #[inline]
    pub fn try_map(&self, handle: &mut Handle<N>) -> bool {
        if let Some(new_handle) = self.map.get(handle) {
            *handle = *new_handle;
            true
        } else {
            false
        }
    }

    /// Tries to map a handle to a handle of its origin. If it exists, the method returns true or false otherwise.
    /// It should be used when you not sure that respective origin exists.
    #[inline]
    pub fn try_map_reflect(&self, reflect_handle: &mut dyn ReflectHandle) -> bool {
        let handle = Handle::new(
            reflect_handle.reflect_index(),
            reflect_handle.reflect_generation(),
        );
        if let Some(new_handle) = self.map.get(&handle) {
            reflect_handle.reflect_set_index(new_handle.index());
            reflect_handle.reflect_set_generation(new_handle.generation());
            true
        } else {
            false
        }
    }

    /// Tries to map each handle in the slice to a handle of its origin. If it exists, the method returns true or false otherwise.
    /// It should be used when you not sure that respective origin exists.
    #[inline]
    pub fn try_map_slice<T>(&self, handles: &mut [T]) -> bool
    where
        T: Deref<Target = Handle<N>> + DerefMut,
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
    #[inline]
    pub fn try_map_silent(&self, inheritable_handle: &mut InheritableVariable<Handle<N>>) -> bool {
        if let Some(new_handle) = self.map.get(inheritable_handle) {
            inheritable_handle.set_value_silent(*new_handle);
            true
        } else {
            false
        }
    }

    /// Returns a shared reference to inner map.
    #[inline]
    pub fn inner(&self) -> &FxHashMap<Handle<N>, Handle<N>> {
        &self.map
    }

    /// Returns inner map.
    #[inline]
    pub fn into_inner(self) -> FxHashMap<Handle<N>, Handle<N>> {
        self.map
    }

    /// Tries to remap handles to nodes in a given entity using reflection. It finds all supported fields recursively
    /// (`Handle<Node>`, `Vec<Handle<Node>>`, `InheritableVariable<Handle<Node>>`, `InheritableVariable<Vec<Handle<Node>>>`)
    /// and automatically maps old handles to new.
    #[inline]
    pub fn remap_handles(&self, node: &mut N, ignored_types: &[TypeId]) {
        let name = node.name().to_string();
        self.remap_handles_any(node, &name, ignored_types);
    }

    pub fn remap_handles_any(
        &self,
        entity: &mut dyn Reflect,
        node_name: &str,
        ignored_types: &[TypeId],
    ) {
        if ignored_types.contains(&(*entity).type_id()) {
            return;
        }

        let mut mapped = false;

        if let Some(handle) = entity.downcast_mut::<Handle<N>>() {
            if handle.is_some() && !self.try_map(handle) {
                Log::warn(format!(
                    "Failed to remap handle {} of node {}!",
                    *handle, node_name
                ));
            }
            mapped = true;
        }

        if mapped {
            return;
        }

        // Handle derived entities handles.
        if let Some(handle) = entity.as_handle_mut() {
            if handle
                .type_info_ref()
                .derived_types
                .contains(&TypeId::of::<N>())
                && handle.reflect_is_some()
                && !self.try_map_reflect(handle)
            {
                Log::warn(format!(
                    "Failed to remap handle {}:{} of node {}!",
                    handle.reflect_index(),
                    handle.reflect_generation(),
                    node_name
                ));
            }
            mapped = true;
        }

        if mapped {
            return;
        }

        if let Some(inheritable) = entity.as_inheritable_variable_mut() {
            // In case of inheritable variable we must take inner value and do not mark variables as modified.
            self.remap_handles_any(inheritable.inner_value_mut(), node_name, ignored_types);

            mapped = true;
        }

        if mapped {
            return;
        }

        if let Some(array) = entity.as_array_mut() {
            // Look in every array item.
            for i in 0..array.reflect_len() {
                // Sparse arrays (like Pool) could have empty entries.
                if let Some(item) = array.reflect_index_mut(i) {
                    self.remap_handles_any(item, node_name, ignored_types);
                }
            }
            mapped = true;
        }

        if mapped {
            return;
        }

        if let Some(hash_map) = entity.as_hash_map_mut() {
            for i in 0..hash_map.reflect_len() {
                if let Some(item) = hash_map.reflect_get_nth_value_mut(i) {
                    self.remap_handles_any(item, node_name, ignored_types);
                }
            }
            mapped = true;
        }

        if mapped {
            return;
        }

        // Continue remapping recursively for every compound field.
        entity.fields_mut(&mut |fields| {
            for field_info_mut in fields {
                self.remap_handles_any(field_info_mut.value, node_name, ignored_types)
            }
        })
    }

    #[inline]
    pub fn remap_inheritable_handles(&self, node: &mut N, ignored_types: &[TypeId]) {
        let name = node.name().to_string();
        self.remap_inheritable_handles_internal(node, &name, false, ignored_types);
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

        if let Some(inheritable) = entity.as_inheritable_variable_mut() {
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

        if mapped {
            return;
        }

        if let Some(handle) = entity.downcast_mut::<Handle<N>>() {
            if do_map && handle.is_some() && !self.try_map(handle) {
                Log::warn(format!(
                    "Failed to remap handle {} of node {}!",
                    *handle, node_name
                ));
            }
            mapped = true;
        }

        if mapped {
            return;
        }

        // Handle derived entities handles.
        if let Some(handle) = entity.as_handle_mut() {
            if do_map
                && handle
                    .type_info_ref()
                    .derived_types
                    .contains(&TypeId::of::<N>())
                && handle.reflect_is_some()
                && !self.try_map_reflect(handle)
            {
                Log::warn(format!(
                    "Failed to remap handle {}:{} of node {}!",
                    handle.reflect_index(),
                    handle.reflect_generation(),
                    node_name
                ));
            }
            mapped = true;
        }

        if mapped {
            return;
        }

        if let Some(array) = entity.as_array_mut() {
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

        if mapped {
            return;
        }

        if let Some(hash_map) = entity.as_hash_map_mut() {
            for i in 0..hash_map.reflect_len() {
                if let Some(item) = hash_map.reflect_get_nth_value_mut(i) {
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

        if mapped {
            return;
        }

        // Continue remapping recursively for every compound field.
        entity.fields_mut(&mut |fields| {
            for field_info_mut in fields {
                self.remap_inheritable_handles_internal(
                    field_info_mut.value,
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

fn reset_property_modified_flag(entity: &mut dyn Reflect, path: &str) {
    entity.resolve_path_mut(path, &mut |result| {
        variable::mark_inheritable_properties_non_modified(
            result.unwrap(),
            &[TypeId::of::<UntypedResource>()],
        );
    })
}

pub trait NodeWrapper: Reflect + NameProvider + Clone {
    type Base: Clone;
    type SceneGraph: SceneGraph<NodeWrapper = Self>;
    type ResourceData: PrefabData<Graph = Self::SceneGraph>;

    /// Returns a shared reference to the instance of the actual node type of this wrapper.
    fn inner_ref(&self) -> &dyn Reflect;

    /// Returns a mutable reference to the instance of the actual node type of this wrapper.
    fn inner_mut(&mut self) -> &mut dyn Reflect;

    /// Returns a shared reference to the common part of all nodes. Typically, it is a structure
    /// that contains node's transform, name, etc.
    fn base(&self) -> &Self::Base;

    /// Set the base of this node.
    fn set_base(&mut self, base: Self::Base);

    fn is_resource_instance_root(&self) -> bool;
    fn original_handle_in_resource(&self) -> Handle<Self>;
    fn set_original_handle_in_resource(&mut self, handle: Handle<Self>);
    fn resource(&self) -> Option<Resource<Self::ResourceData>>;
    fn self_handle(&self) -> Handle<Self>;
    fn parent(&self) -> Handle<Self>;
    fn children(&self) -> &[Handle<Self>];
    fn children_mut(&mut self) -> &mut [Handle<Self>];
    fn instance_id(&self) -> Uuid;

    /// Puts the given `child` handle to the given position `pos`, by swapping positions.
    #[inline]
    fn swap_child_position(&mut self, child: Handle<Self>, pos: usize) -> Option<usize> {
        let children = self.children_mut();

        if pos >= children.len() {
            return None;
        }

        if let Some(current_position) = children.iter().position(|c| *c == child) {
            children.swap(current_position, pos);

            Some(current_position)
        } else {
            None
        }
    }

    #[inline]
    fn set_child_position(&mut self, child: Handle<Self>, dest_pos: usize) -> Option<usize> {
        let children = self.children_mut();

        if dest_pos >= children.len() {
            return None;
        }

        if let Some(mut current_position) = children.iter().position(|c| *c == child) {
            let prev_position = current_position;

            match current_position.cmp(&dest_pos) {
                Ordering::Less => {
                    while current_position != dest_pos {
                        let next = current_position.saturating_add(1);
                        children.swap(current_position, next);
                        current_position = next;
                    }
                }
                Ordering::Equal => {}
                Ordering::Greater => {
                    while current_position != dest_pos {
                        let prev = current_position.saturating_sub(1);
                        children.swap(current_position, prev);
                        current_position = prev;
                    }
                }
            }

            Some(prev_position)
        } else {
            None
        }
    }

    #[inline]
    fn child_position(&self, child: Handle<Self>) -> Option<usize> {
        self.children().iter().position(|c| *c == child)
    }

    #[inline]
    fn has_child(&self, child: Handle<Self>) -> bool {
        self.children().contains(&child)
    }

    fn revert_inheritable_property(&mut self, path: &str) -> Option<Box<dyn Reflect>> {
        let mut previous_value = None;

        // Revert only if there's parent resource (the node is an instance of some resource).
        if let Some(resource) = self.resource().as_ref() {
            let resource_data = resource.data_ref();
            let parent = &resource_data
                .graph()
                .node(self.original_handle_in_resource());

            let mut parent_value = None;

            // Find and clone parent's value first.
            parent
                .inner_ref()
                .resolve_path(path, &mut |result| match result {
                    Ok(parent_field) => {
                        if let Some(parent_inheritable) = parent_field.as_inheritable_variable() {
                            parent_value = parent_inheritable.inner_value_ref().try_clone_box();
                        }
                    }
                    Err(e) => Log::err(format!(
                        "Failed to resolve parent path {path}. Reason: {e:?}"
                    )),
                });

            // Check whether the child's field is inheritable and modified.
            let mut need_revert = false;

            self.inner_mut()
                .resolve_path_mut(path, &mut |result| match result {
                    Ok(child_field) => {
                        if let Some(child_inheritable) = child_field.as_inheritable_variable_mut() {
                            need_revert = child_inheritable.is_modified();
                        } else {
                            Log::err(format!("Property {path} is not inheritable!"))
                        }
                    }
                    Err(e) => Log::err(format!(
                        "Failed to resolve child path {path}. Reason: {e:?}"
                    )),
                });

            // Try to apply it to the child.
            if need_revert {
                if let Some(parent_value) = parent_value {
                    let mut was_set = false;

                    let mut parent_value = Some(parent_value);

                    self.inner_mut().set_field_by_path(
                        path,
                        parent_value.take().unwrap(),
                        &mut |result| match result {
                            Ok(old_value) => {
                                previous_value = Some(old_value);

                                was_set = true;
                            }
                            Err(_) => Log::err(format!(
                                "Failed to revert property {path}. Reason: no such property!"
                            )),
                        },
                    );

                    if was_set {
                        // Reset modified flag.
                        reset_property_modified_flag(self.inner_mut(), path);
                    }
                }
            }
        }

        previous_value
    }

    /// Tries to downcast self to the specified type, or if it is not possible, tries to find a
    /// field of the specified type.
    #[inline]
    fn self_or_field_ref<T: Reflect>(&self) -> Option<&T> {
        self.inner_ref().self_or_field_ref::<T>()
    }

    /// Tries to downcast self to the specified type, or if it is not possible, tries to find a
    /// field of the specified type.
    #[inline]
    fn self_or_field_mut<T: Reflect>(&mut self) -> Option<&mut T> {
        self.inner_mut().self_or_field_mut::<T>()
    }

    /// Checks if the node is an instance of the given type or has a field of the same type.
    #[inline]
    fn is_or_has_field<T: Reflect>(&self) -> bool {
        self.self_or_field_ref::<T>().is_some()
    }
}

pub trait PrefabData: TypedResourceData + 'static {
    type Graph: SceneGraph;

    fn graph(&self) -> &Self::Graph;
    fn mapping(&self) -> NodeMapping;
}

#[derive(Debug)]
pub struct LinkScheme<N> {
    pub root: Handle<N>,
    pub links: Vec<(Handle<N>, Handle<N>)>,
}

impl<N> Default for LinkScheme<N> {
    fn default() -> Self {
        Self {
            root: Default::default(),
            links: Default::default(),
        }
    }
}

/// SceneGraph is a trait for all scene graphs to implement.
pub trait SceneGraph: 'static {
    type Prefab: PrefabData<Graph = Self>;
    type NodeWrapper: NodeWrapper<SceneGraph = Self, ResourceData = Self::Prefab>;

    /// Generate a string that briefly summarizes the content of the graph for debugging.
    fn summary(&self) -> String;

    /// Returns actual type id of the node.
    fn actual_type_id(&self, handle: Handle<Self::NodeWrapper>) -> Result<TypeId, PoolError>;

    /// Returns actual type name of the node.
    fn actual_type_name(
        &self,
        handle: Handle<Self::NodeWrapper>,
    ) -> Result<&'static str, PoolError>;

    /// Returns a list of derived type ids of the node.
    fn derived_type_ids(&self, handle: Handle<Self::NodeWrapper>)
        -> Result<Vec<TypeId>, PoolError>;

    /// Returns a handle of the root node of the graph.
    fn root(&self) -> Handle<Self::NodeWrapper>;

    /// Sets the new root of the graph. If used incorrectly, it may create isolated sub-graphs.
    fn set_root(&mut self, root: Handle<Self::NodeWrapper>);

    /// Tries to borrow a node, returns Ok(node) if the handle is valid, Err(...) - otherwise.
    fn try_get_node(
        &self,
        handle: Handle<Self::NodeWrapper>,
    ) -> Result<&Self::NodeWrapper, PoolError>;

    /// Tries to borrow a node, returns Ok(node) if the handle is valid, Err(...) - otherwise.
    fn try_get_node_mut(
        &mut self,
        handle: Handle<Self::NodeWrapper>,
    ) -> Result<&mut Self::NodeWrapper, PoolError>;

    /// Checks whether the given node handle is valid or not.
    fn is_valid_handle(&self, handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>) -> bool;

    /// Adds a new node to the graph.
    fn add_node(&mut self, node: Self::NodeWrapper) -> Handle<Self::NodeWrapper>;

    /// Adds a new node to the graph at the given handle.
    fn add_node_at_handle(&mut self, node: Self::NodeWrapper, handle: Handle<Self::NodeWrapper>);

    /// Destroys the node and its children recursively.
    fn remove_node(&mut self, node_handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>);

    /// Links specified child with specified parent.
    fn link_nodes(
        &mut self,
        child: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        parent: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
    );

    /// Unlinks specified node from its parent and attaches it to root graph node.
    fn unlink_node(&mut self, node_handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>);

    /// Detaches the node from its parent, making the node unreachable from any other node in the
    /// graph.
    fn isolate_node(&mut self, node_handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>);

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    fn pair_iter(&self) -> impl Iterator<Item = (Handle<Self::NodeWrapper>, &Self::NodeWrapper)>;

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    fn linear_iter(&self) -> impl Iterator<Item = &Self::NodeWrapper>;

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    fn linear_iter_mut(&mut self) -> impl Iterator<Item = &mut Self::NodeWrapper>;

    /// Tries to borrow a graph node by the given handle. This method accepts both type-agnostic
    /// container (read - [`Self::NodeWrapper`]) handles and the actual node type handles. For example,
    /// if the container type is `Node` (holds `Box<dyn NodeTrait>`) and there's a `SpecificNode`
    /// (implements the `NodeTrait`), then this method can accept both `Handle<Node>` and
    /// `Handle<SpecificNode>`. Internally, this method will try to downcast the container to
    /// the specified type (which may fail). If the downcast is failed, then the implementation will
    /// also search for the field of the specified type in the node.
    fn try_get<U: ObjectOrVariant<Self::NodeWrapper>>(
        &self,
        handle: Handle<U>,
    ) -> Result<&U, PoolError>;

    /// Tries to borrow a graph node by the given handle. This method accepts both type-agnostic
    /// container (read - [`Self::NodeWrapper`]) handles and the actual node type handles. For example,
    /// if the container type is `Node` (holds `Box<dyn NodeTrait>`) and there's a `SpecificNode`
    /// (implements the `NodeTrait`), then this method can accept both `Handle<Node>` and
    /// `Handle<SpecificNode>`. Internally, this method will try to downcast the container to
    /// the specified type (which may fail). If the downcast is failed, then the implementation will
    /// also search for the field of the specified type in the node.
    fn try_get_mut<U: ObjectOrVariant<Self::NodeWrapper>>(
        &mut self,
        handle: Handle<U>,
    ) -> Result<&mut U, PoolError>;

    /// Borrows a node by its handle.
    fn node(&self, handle: Handle<Self::NodeWrapper>) -> &Self::NodeWrapper {
        self.try_get_node(handle).unwrap()
    }

    /// Borrows a node by its handle.
    fn node_mut(&mut self, handle: Handle<Self::NodeWrapper>) -> &mut Self::NodeWrapper {
        self.try_get_node_mut(handle).unwrap()
    }

    /// Tries to borrow a node, returns Some(node) if the uuid is valid, None - otherwise.
    fn try_get_node_by_uuid(&self, uuid: Uuid) -> Option<&Self::NodeWrapper> {
        self.linear_iter().find(|n| n.instance_id() == uuid)
    }

    /// Tries to borrow a node, returns Some(node) if the uuid is valid, None - otherwise.
    fn try_get_node_by_uuid_mut(&mut self, uuid: Uuid) -> Option<&mut Self::NodeWrapper> {
        self.linear_iter_mut().find(|n| n.instance_id() == uuid)
    }

    /// Reorders the node hierarchy so the `new_root` becomes the root node for the entire hierarchy
    /// under the `prev_root` node. For example, if we have this hierarchy and want to set `C` as
    /// the new root:
    ///
    /// ```text
    /// Root_
    ///      |_A_
    ///          |_B
    ///          |_C_
    ///             |_D
    /// ```
    ///
    /// The new hierarchy will become:
    ///
    /// ```text
    /// C_
    ///   |_D
    ///   |_A_
    ///   |   |_B
    ///   |_Root
    /// ```
    ///
    /// This method returns an instance of [`LinkScheme`], that could be used to revert the hierarchy
    /// back to its original. See [`Self::apply_link_scheme`] for more info.
    #[inline]
    #[allow(clippy::unnecessary_to_owned)] // False-positive
    fn change_hierarchy_root(
        &mut self,
        prev_root: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        new_root: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
    ) -> LinkScheme<Self::NodeWrapper> {
        let prev_root = prev_root.to_base();
        let new_root = new_root.to_base();

        let mut scheme = LinkScheme::default();

        if prev_root == new_root {
            return scheme;
        }

        scheme
            .links
            .push((prev_root, self.node(prev_root).parent()));

        scheme.links.push((new_root, self.node(new_root).parent()));

        self.isolate_node(new_root);

        for prev_root_child in self.node(prev_root).children().to_vec() {
            self.link_nodes(prev_root_child, new_root);
            scheme.links.push((prev_root_child, prev_root));
        }

        self.link_nodes(prev_root, new_root);

        if prev_root == self.root() {
            self.set_root(new_root);
            scheme.root = prev_root;
        } else {
            scheme.root = self.root();
        }

        scheme
    }

    /// Applies the given link scheme to the graph, basically reverting graph structure to the one
    /// that was before the call of [`Self::change_hierarchy_root`].
    #[inline]
    fn apply_link_scheme(&mut self, mut scheme: LinkScheme<Self::NodeWrapper>) {
        for (child, parent) in scheme.links.drain(..) {
            if parent.is_some() {
                self.link_nodes(child, parent);
            } else {
                self.isolate_node(child);
            }
        }
        if scheme.root.is_some() {
            self.set_root(scheme.root);
        }
    }

    /// Removes all the nodes from the given slice.
    #[inline]
    fn remove_nodes(&mut self, nodes: &[Handle<impl ObjectOrVariant<Self::NodeWrapper>>]) {
        for &node in nodes {
            if self.is_valid_handle(node) {
                self.remove_node(node)
            }
        }
    }

    /// Tries to borrow a node at the given handle and downcast it to the specified type. If downcasting
    /// it is not possible, tries to find a field of the specified type.
    #[inline]
    fn try_get_of_type<T>(&self, handle: Handle<Self::NodeWrapper>) -> Result<&T, PoolError>
    where
        T: Reflect,
    {
        self.try_get_node(handle).and_then(|n| {
            n.inner_ref()
                .self_or_field_ref()
                .ok_or(PoolError::NoSuchField(handle.into()))
        })
    }

    /// Tries to mutably borrow a node at the given handle and downcast it to the specified type. If downcasting
    /// it is not possible, tries to find a field of the specified type.
    #[inline]
    fn try_get_mut_of_type<T>(
        &mut self,
        handle: Handle<Self::NodeWrapper>,
    ) -> Result<&mut T, PoolError>
    where
        T: Reflect,
    {
        self.try_get_node_mut(handle).and_then(|n| {
            n.inner_mut()
                .self_or_field_mut()
                .ok_or(PoolError::NoSuchField(handle.into()))
        })
    }

    /// Tries to borrow a node by the given handle and checks if it is possible to downcast the node
    /// to the specified type. If the downcasting is not possible, this method tries to find a field
    /// of the given type.
    #[inline]
    fn is_or_has_field<T>(&self, handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>) -> bool
    where
        T: Reflect,
    {
        self.try_get_of_type::<T>(handle.to_base()).is_ok()
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    fn find_map<C, T>(
        &self,
        root_node: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        cmp: &mut C,
    ) -> Option<(Handle<Self::NodeWrapper>, &T)>
    where
        C: FnMut(&Self::NodeWrapper) -> Option<&T>,
        T: ?Sized,
    {
        let root_node = root_node.to_base();
        self.try_get_node(root_node).ok().and_then(|root| {
            if let Some(x) = cmp(root) {
                Some((root_node, x))
            } else {
                root.children().iter().find_map(|c| self.find_map(*c, cmp))
            }
        })
    }

    /// Searches for a node up the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    fn find_up<C>(
        &self,
        root_node: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        cmp: &mut C,
    ) -> Option<(Handle<Self::NodeWrapper>, &Self::NodeWrapper)>
    where
        C: FnMut(&Self::NodeWrapper) -> bool,
    {
        let mut handle = root_node.to_base();
        while let Ok(node) = self.try_get_node(handle) {
            if cmp(node) {
                return Some((handle, node));
            }
            handle = node.parent();
        }
        None
    }

    /// The same as [`Self::find_up`], but only returns node handle which will be [`Handle::NONE`]
    /// if nothing is found.
    #[inline]
    fn find_handle_up<C>(
        &self,
        root_node: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        cmp: &mut C,
    ) -> Handle<Self::NodeWrapper>
    where
        C: FnMut(&Self::NodeWrapper) -> bool,
    {
        self.find_up(root_node, cmp)
            .map(|(h, _)| h)
            .unwrap_or_default()
    }

    #[inline]
    fn find_self_or_field_up<T>(
        &self,
        node_handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
    ) -> Option<(Handle<Self::NodeWrapper>, &T)>
    where
        T: Reflect,
    {
        self.find_up_map(node_handle, &mut |node| {
            node.inner_ref().self_or_field_ref::<T>()
        })
    }

    #[inline]
    fn find_self_or_field<T>(
        &self,
        node_handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
    ) -> Option<(Handle<Self::NodeWrapper>, &T)>
    where
        T: Reflect,
    {
        self.find_map(node_handle, &mut |node| {
            node.inner_ref().self_or_field_ref::<T>()
        })
    }

    /// Searches for a node up the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    fn find_up_map<C, T>(
        &self,
        root_node: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        cmp: &mut C,
    ) -> Option<(Handle<Self::NodeWrapper>, &T)>
    where
        C: FnMut(&Self::NodeWrapper) -> Option<&T>,
        T: ?Sized,
    {
        let mut handle = root_node.to_base();
        while let Ok(node) = self.try_get_node(handle) {
            if let Some(x) = cmp(node) {
                return Some((handle, x));
            }
            handle = node.parent();
        }
        None
    }

    /// Searches for a node with the specified name down the tree starting from the specified node. Returns a tuple with
    /// a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    fn find_by_name(
        &self,
        root_node: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        name: &str,
    ) -> Option<(Handle<Self::NodeWrapper>, &Self::NodeWrapper)> {
        self.find(root_node, &mut |node| node.name() == name)
    }

    /// Searches for a node with the specified name up the tree starting from the specified node. Returns a tuple with a
    /// handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    fn find_up_by_name(
        &self,
        root_node: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        name: &str,
    ) -> Option<(Handle<Self::NodeWrapper>, &Self::NodeWrapper)> {
        self.find_up(root_node, &mut |node| node.name() == name)
    }

    /// Searches for a node with the specified name down the tree starting from the graph root. Returns a tuple with a
    /// handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    fn find_by_name_from_root(
        &self,
        name: &str,
    ) -> Option<(Handle<Self::NodeWrapper>, &Self::NodeWrapper)> {
        self.find_by_name(self.root(), name)
    }

    #[inline]
    fn find_handle_by_name_from_root(&self, name: &str) -> Handle<Self::NodeWrapper> {
        self.find_by_name(self.root(), name)
            .map(|(h, _)| h)
            .unwrap_or_default()
    }

    /// Searches node using specified compare closure starting from root. Returns a tuple with a handle and
    /// a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    fn find_from_root<C>(
        &self,
        cmp: &mut C,
    ) -> Option<(Handle<Self::NodeWrapper>, &Self::NodeWrapper)>
    where
        C: FnMut(&Self::NodeWrapper) -> bool,
    {
        self.find(self.root(), cmp)
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure.
    /// Returns a tuple with a handle and a reference to the found node. If nothing is found, it
    /// returns [`None`].
    #[inline]
    fn find<C>(
        &self,
        root_node: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        cmp: &mut C,
    ) -> Option<(Handle<Self::NodeWrapper>, &Self::NodeWrapper)>
    where
        C: FnMut(&Self::NodeWrapper) -> bool,
    {
        let root_node = root_node.to_base();
        self.try_get_node(root_node).ok().and_then(|root| {
            if cmp(root) {
                Some((root_node, root))
            } else {
                root.children().iter().find_map(|c| self.find(*c, cmp))
            }
        })
    }

    /// The same as [`Self::find`], but only returns node handle which will be [`Handle::NONE`]
    /// if nothing is found.
    #[inline]
    fn find_handle<C>(
        &self,
        root_node: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        cmp: &mut C,
    ) -> Handle<Self::NodeWrapper>
    where
        C: FnMut(&Self::NodeWrapper) -> bool,
    {
        self.find(root_node, cmp)
            .map(|(h, _)| h)
            .unwrap_or_default()
    }

    /// Returns position of the node in its parent children list and the handle to the parent. Adds
    /// given `offset` to the position. For example, if you have the following hierarchy:
    ///
    /// ```text
    /// A_
    ///  |B
    ///  |C
    /// ```
    ///
    /// Calling this method with a handle of `C` will return `Some((handle_of(A), 1))`. The returned
    /// value will be clamped in the `0..parent_child_count` range. `None` will be returned only if
    /// the given handle is invalid, or it is the root node.
    #[inline]
    fn relative_position(
        &self,
        child: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        offset: isize,
    ) -> Option<(Handle<Self::NodeWrapper>, usize)> {
        let child = child.to_base();
        let parents_parent_handle = self.try_get_node(child).ok()?.parent();
        let parents_parent_ref = self.try_get_node(parents_parent_handle).ok()?;
        let position = parents_parent_ref.child_position(child)?;
        Some((
            parents_parent_handle,
            ((position as isize + offset) as usize).clamp(0, parents_parent_ref.children().len()),
        ))
    }

    /// Create a graph depth traversal iterator.
    #[inline]
    fn traverse_iter(
        &self,
        from: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
    ) -> impl Iterator<Item = (Handle<Self::NodeWrapper>, &Self::NodeWrapper)> {
        GraphTraverseIteratorRef {
            graph: self,
            start_node_handle: from.to_base(),
            current_node_handle: from.to_base(),
            index: None,
        }
    }

    /// Create a graph depth traversal iterator that yields a pair of the node handle and the mutable
    /// reference to this node.
    #[inline]
    fn traverse_iter_mut(
        &mut self,
        from: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
    ) -> impl Iterator<Item = (Handle<Self::NodeWrapper>, &mut Self::NodeWrapper)> {
        GraphTraverseIteratorMut {
            graph: self,
            start_node_handle: from.to_base(),
            current_node_handle: from.to_base(),
            index: None,
        }
    }

    /// Create a graph depth traversal iterator.
    #[inline]
    fn traverse_handle_iter(
        &self,
        from: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
    ) -> impl Iterator<Item = Handle<Self::NodeWrapper>> {
        self.traverse_iter(from).map(|(handle, _)| handle)
    }

    /// This method checks integrity of the graph and restores it if needed. For example, if a node
    /// was added in a parent asset, then it must be added in the graph. Alternatively, if a node was
    /// deleted in a parent asset, then its instance must be deleted in the graph.
    fn restore_integrity<F>(
        &mut self,
        mut instantiate: F,
    ) -> Vec<(Handle<Self::NodeWrapper>, Resource<Self::Prefab>)>
    where
        F: FnMut(
            Resource<Self::Prefab>,
            &Self::Prefab,
            Handle<Self::NodeWrapper>,
            &mut Self,
        ) -> (Handle<Self::NodeWrapper>, NodeHandleMap<Self::NodeWrapper>),
    {
        Log::writeln(MessageKind::Information, "Checking integrity...");

        let instances = self
            .pair_iter()
            .filter_map(|(h, n)| {
                if n.is_resource_instance_root() {
                    Some((h, n.resource().unwrap()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let instance_count = instances.len();
        let mut restored_count = 0;

        for (instance_root, resource) in instances.iter().cloned() {
            // Step 1. Find and remove orphaned nodes.
            let mut nodes_to_delete = Vec::new();
            for (_, node) in self.traverse_iter(instance_root) {
                if let Some(resource) = node.resource() {
                    if let Some(model) = resource.state().data() {
                        if !model
                            .graph()
                            .is_valid_handle(node.original_handle_in_resource())
                        {
                            nodes_to_delete.push(node.self_handle());

                            Log::warn(format!(
                                "Node {} ({}:{}) and its children will be deleted, because it \
                    does not exist in the parent asset!",
                                node.name(),
                                node.self_handle().index(),
                                node.self_handle().generation(),
                            ))
                        }
                    } else {
                        Log::warn(format!(
                            "Node {} ({}:{}) and its children will be deleted, because its \
                    parent asset failed to load!",
                            node.name(),
                            node.self_handle().index(),
                            node.self_handle().generation(),
                        ))
                    }
                }
            }

            for node_to_delete in nodes_to_delete {
                if self.is_valid_handle(node_to_delete) {
                    self.remove_node(node_to_delete);
                }
            }

            // Step 2. Look for missing nodes and create appropriate instances for them.
            let mut model = resource.state();
            let model_kind = model.kind();
            if let Some(data) = model.data() {
                let resource_graph = data.graph();

                let Ok(resource_instance_root) = self
                    .try_get_node(instance_root)
                    .map(|n| n.original_handle_in_resource())
                else {
                    continue;
                };

                if resource_instance_root.is_none() {
                    let instance = self.node(instance_root);
                    Log::writeln(
                        MessageKind::Warning,
                        format!(
                            "There is an instance of resource {} \
                    but original node {} cannot be found!",
                            model_kind,
                            instance.name()
                        ),
                    );

                    continue;
                }

                let mut traverse_stack = vec![resource_instance_root];
                while let Some(resource_node_handle) = traverse_stack.pop() {
                    let resource_node = resource_graph.node(resource_node_handle);

                    // Root of the resource is not belongs to resource, it is just a convenient way of
                    // consolidation all descendants under a single node.
                    let mut compare = |n: &Self::NodeWrapper| {
                        n.original_handle_in_resource() == resource_node_handle
                    };

                    if resource_node_handle != resource_graph.root()
                        && self.find(instance_root, &mut compare).is_none()
                    {
                        Log::writeln(
                            MessageKind::Warning,
                            format!(
                                "Instance of node {} is missing. Restoring integrity...",
                                resource_node.name()
                            ),
                        );

                        // Instantiate missing node.
                        let (copy, old_to_new_mapping) =
                            instantiate(resource.clone(), data, resource_node_handle, self);

                        restored_count += old_to_new_mapping.inner().len();

                        // Link it with existing node.
                        if resource_node.parent().is_some() {
                            let parent = self.find(instance_root, &mut |n| {
                                n.original_handle_in_resource() == resource_node.parent()
                            });

                            if let Some((parent_handle, _)) = parent {
                                self.link_nodes(copy, parent_handle);
                            } else {
                                // Fail-safe route - link with root of instance.
                                self.link_nodes(copy, instance_root);
                            }
                        } else {
                            // Fail-safe route - link with root of instance.
                            self.link_nodes(copy, instance_root);
                        }
                    }

                    traverse_stack.extend_from_slice(resource_node.children());
                }
            }
        }

        Log::writeln(
            MessageKind::Information,
            format!(
                "Integrity restored for {instance_count} instances! {restored_count} new nodes were added!"
            ),
        );

        instances
    }

    // Iterate through every node and try to find a corresponding node in the parent resource for that node.
    // If a node has a parent resource but no correspoding node in the parent resource, then delete the node.
    // Otherwise, use reflection to make the unmodified variables of the node match the values in the corresponding node.
    fn restore_original_handles_and_inherit_properties<F>(
        &mut self,
        ignored_types: &[TypeId],
        mut before_inherit: F,
    ) where
        F: FnMut(&Self::NodeWrapper, &mut Self::NodeWrapper),
    {
        let mut ignored_types = ignored_types.to_vec();
        // Do not try to inspect resources, because it most likely cause a deadlock.
        ignored_types.push(TypeId::of::<UntypedResource>());

        // Iterate over each node in the graph and resolve original handles. Original handle is a handle
        // to a node in resource from which a node was instantiated from. Also sync inheritable properties
        // if needed.
        for node in self.linear_iter_mut() {
            if let Some(model) = node.resource() {
                let mut header = model.state();
                let model_kind = header.kind();
                if let Some(data) = header.data() {
                    let resource_graph = data.graph();

                    let resource_node = match data.mapping() {
                        NodeMapping::UseNames => {
                            // For some models we can resolve it only by names of nodes, but this is not
                            // reliable way of doing this, because some editors allow nodes to have same
                            // names for objects, but here we'll assume that modellers will not create
                            // models with duplicated names and user of the engine reads log messages.
                            resource_graph
                                .pair_iter()
                                .find_map(|(handle, resource_node)| {
                                    if resource_node.name() == node.name() {
                                        Some((resource_node, handle))
                                    } else {
                                        None
                                    }
                                })
                        }
                        NodeMapping::UseHandles => {
                            // Use original handle directly.
                            resource_graph
                                .try_get_node(node.original_handle_in_resource())
                                .map(|resource_node| {
                                    (resource_node, node.original_handle_in_resource())
                                })
                                .ok()
                        }
                    };

                    if let Some((resource_node, original)) = resource_node {
                        node.set_original_handle_in_resource(original);

                        before_inherit(resource_node, node);

                        // Check if the actual node types (this and parent's) are equal, and if not - copy the
                        // node and replace its base.
                        if node.inner_ref().type_id() != resource_node.inner_ref().type_id() {
                            Log::warn(format!(
                                "Node {}({}:{}) instance \
                                        have different type than in the respective parent \
                                        asset {}. The type will be fixed.",
                                node.name(),
                                node.self_handle().index(),
                                node.self_handle().generation(),
                                model_kind
                            ));

                            let base = node.base().clone();
                            let mut resource_node_clone = resource_node.clone();
                            variable::mark_inheritable_properties_non_modified(
                                &mut resource_node_clone as &mut dyn Reflect,
                                &ignored_types,
                            );
                            resource_node_clone.set_base(base);
                            *node = resource_node_clone;
                        }

                        // Then try to inherit properties.
                        Log::verify(variable::try_inherit_properties(
                            node,
                            resource_node,
                            &ignored_types,
                        ));
                    } else {
                        Log::warn(format!(
                            "Unable to find original handle for node {}. The node will be removed!",
                            node.name(),
                        ))
                    }
                }
            }
        }

        Log::writeln(MessageKind::Information, "Original handles resolved!");
    }

    /// Maps handles in properties of instances after property inheritance. It is needed, because when a
    /// property contains node handle, the handle cannot be used directly after inheritance. Instead, it
    /// must be mapped to respective instance first.
    ///
    /// To do so, we at first, build node handle mapping (original handle -> instance handle) starting from
    /// instance root. Then we must find all inheritable properties and try to remap them to instance handles.
    fn remap_handles(&mut self, instances: &[(Handle<Self::NodeWrapper>, Resource<Self::Prefab>)]) {
        for (instance_root, resource) in instances {
            // Prepare old -> new handle mapping first by walking over the graph
            // starting from instance root.
            let mut old_new_mapping = NodeHandleMap::default();
            let mut traverse_stack = vec![*instance_root];
            while let Some(node_handle) = traverse_stack.pop() {
                let Ok(node) = self.try_get_node(node_handle) else {
                    continue;
                };
                if let Some(node_resource) = node.resource().as_ref() {
                    // We're interested only in instance nodes.
                    if node_resource == resource {
                        let previous_mapping =
                            old_new_mapping.insert(node.original_handle_in_resource(), node_handle);
                        // There should be no such node.
                        if previous_mapping.is_some() {
                            Log::warn(format!(
                                "There are multiple original nodes for {:?}! Previous was {:?}. \
                                This can happen if a respective node was deleted.",
                                node_handle,
                                node.original_handle_in_resource()
                            ))
                        }
                    }
                }

                traverse_stack.extend_from_slice(node.children());
            }

            // Lastly, remap handles. We can't do this in single pass because there could
            // be cross references.
            for (_, handle) in old_new_mapping.inner().iter() {
                old_new_mapping.remap_inheritable_handles(
                    self.node_mut(*handle),
                    &[TypeId::of::<UntypedResource>()],
                );
            }
        }
    }
}

/// Iterator that traverses tree in depth and returns shared references to nodes.
pub struct GraphTraverseIteratorRef<'a, G: ?Sized, N> {
    graph: &'a G,
    start_node_handle: Handle<N>,
    current_node_handle: Handle<N>,
    index: Option<usize>,
}

impl<'a, G: ?Sized, N> Iterator for GraphTraverseIteratorRef<'a, G, N>
where
    G: SceneGraph<NodeWrapper = N>,
    N: NodeWrapper,
{
    type Item = (Handle<N>, &'a N);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let current_node_handle = self.current_node_handle;
        match self.graph.try_get(current_node_handle) {
            Ok(current_node) => {
                match current_node.children().first() {
                    Some(first) => {
                        self.current_node_handle = *first;
                    }
                    None => {
                        let mut parent_handle = current_node.parent();
                        while let Ok(parent) = self.graph.try_get(parent_handle) {
                            let parent_children = parent.children();
                            let index = self.index.get_or_insert_with(|| {
                                parent_children
                                    .iter()
                                    .position(|h| *h == self.current_node_handle)
                                    .expect("must be in parent's list")
                            });
                            *index += 1;
                            if let Some(next_child_handle) = parent_children.get(*index) {
                                self.current_node_handle = *next_child_handle;
                                return Some((current_node_handle, current_node));
                            } else {
                                self.current_node_handle = parent_handle;
                                self.index = None;
                                parent_handle = parent.parent();
                                if self.current_node_handle == self.start_node_handle {
                                    break;
                                }
                            }
                        }
                        self.current_node_handle = Handle::NONE;
                        self.index = None;
                    }
                }
                Some((current_node_handle, current_node))
            }
            Err(_) => None,
        }
    }
}

/// Iterator that traverses tree in depth and returns shared references to nodes.
pub struct GraphTraverseIteratorMut<'a, G: ?Sized, N> {
    graph: &'a mut G,
    start_node_handle: Handle<N>,
    current_node_handle: Handle<N>,
    index: Option<usize>,
}

impl<'a, G: ?Sized, N> Iterator for GraphTraverseIteratorMut<'a, G, N>
where
    G: SceneGraph<NodeWrapper = N>,
    N: NodeWrapper,
{
    type Item = (Handle<N>, &'a mut N);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let current_node_handle = self.current_node_handle;

        // SAFETY: This is safe, because we're borrowing the nodes directly from the graph.
        // By doing this transmute we're essentially telling the compiler that the 'a lifetime
        // is valid for the returned items.
        let graph = unsafe { std::mem::transmute::<&'_ mut G, &'a mut G>(self.graph) };

        match graph.try_get_mut(current_node_handle) {
            Ok(current_node) => {
                match current_node.children().first() {
                    Some(first) => {
                        self.current_node_handle = *first;
                    }
                    None => {
                        let mut parent_handle = current_node.parent();
                        while let Ok(parent) = graph.try_get(parent_handle) {
                            let parent_children = parent.children();
                            let index = self.index.get_or_insert_with(|| {
                                parent_children
                                    .iter()
                                    .position(|h| *h == self.current_node_handle)
                                    .expect("must be in parent's list")
                            });
                            *index += 1;
                            if let Some(next_child_handle) = parent_children.get(*index) {
                                self.current_node_handle = *next_child_handle;
                                return Some((
                                    current_node_handle,
                                    graph
                                        .try_get_mut(current_node_handle)
                                        .expect("must be valid"),
                                ));
                            } else {
                                self.current_node_handle = parent_handle;
                                self.index = None;
                                parent_handle = parent.parent();
                                if self.current_node_handle == self.start_node_handle {
                                    break;
                                }
                            }
                        }
                        self.current_node_handle = Handle::NONE;
                        self.index = None;
                    }
                }
                Some((
                    current_node_handle,
                    graph
                        .try_get_mut(current_node_handle)
                        .expect("must be valid"),
                ))
            }
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{NodeHandleMap, NodeMapping, NodeWrapper, PrefabData, SceneGraph};
    use fyrox_core::pool::{ObjectOrVariant, ObjectOrVariantHelper, PoolError};
    use fyrox_core::uuid::Uuid;
    use fyrox_core::{
        define_as_any_trait,
        pool::{Handle, PayloadContainer, Pool},
        reflect::prelude::*,
        visitor::prelude::*,
        NameProvider,
    };
    use fyrox_resource::{untyped::UntypedResource, Resource, ResourceData};
    use std::marker::PhantomData;
    use std::{
        any::{Any, TypeId},
        error::Error,
        fmt::Debug,
        ops::{Deref, DerefMut, Index, IndexMut},
        path::Path,
    };

    #[derive(Visit, Reflect, PartialEq, Debug, Clone)]
    #[reflect(type_uuid = "34f5e3ea-3008-45e9-9e6e-0b8fbff1ac28")]
    pub struct Base {
        name: String,
        self_handle: Handle<Node>,
        is_resource_instance_root: bool,
        original_handle_in_resource: Handle<Node>,
        resource: Option<Resource<Graph>>,
        parent: Handle<Node>,
        children: Vec<Handle<Node>>,
        instance_id: Uuid,
    }

    impl Default for Base {
        fn default() -> Self {
            Self {
                name: Default::default(),
                self_handle: Default::default(),
                is_resource_instance_root: Default::default(),
                original_handle_in_resource: Default::default(),
                resource: Default::default(),
                parent: Default::default(),
                children: Default::default(),
                instance_id: Uuid::new_v4(),
            }
        }
    }

    /// A set of useful methods that is possible to auto-implement.
    pub trait BaseNodeTrait: Any + Debug + Deref<Target = Base> + DerefMut + Send {
        /// This method creates raw copy of a node, it should never be called in normal circumstances
        /// because internally nodes may (and most likely will) contain handles to other nodes. To
        /// correctly clone a node you have to use [copy_node](struct.Graph.html#method.copy_node).
        fn clone_box(&self) -> Node;
    }

    impl<T> BaseNodeTrait for T
    where
        T: Clone + NodeTrait + 'static,
    {
        fn clone_box(&self) -> Node {
            Node(Box::new(self.clone()))
        }
    }

    define_as_any_trait!(NodeAsAny => NodeTrait);

    pub trait NodeTrait: BaseNodeTrait + Reflect + Visit + NodeAsAny {}

    // Essentially implements ObjectOrVariant for NodeTrait types.
    // See ObjectOrVariantHelper for the cause of the indirection.
    impl<T: NodeTrait> ObjectOrVariantHelper<Node, T> for PhantomData<T> {
        fn convert_to_dest_type_helper(node: &Node) -> Option<&T> {
            NodeAsAny::as_any(node.0.deref()).downcast_ref()
        }
        fn convert_to_dest_type_helper_mut(node: &mut Node) -> Option<&mut T> {
            NodeAsAny::as_any_mut(node.0.deref_mut()).downcast_mut()
        }
    }

    #[derive(Debug, Reflect)]
    #[reflect(type_uuid = "e2c18cd6-f1bf-40e5-9ec6-0dc48cc75409")]
    pub struct Node(#[reflect(deref, display_name = "Node")] Box<dyn NodeTrait>);

    impl Clone for Node {
        fn clone(&self) -> Self {
            self.0.clone_box()
        }
    }

    impl PartialEq for Node {
        fn eq(&self, other: &Self) -> bool {
            self.0.try_compare(other.0.deref()).unwrap_or_default()
        }
    }

    impl Node {
        fn new(node: impl NodeTrait) -> Self {
            Self(Box::new(node))
        }
    }

    impl Visit for Node {
        fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
            self.0.visit(name, visitor)
        }
    }

    impl NameProvider for Node {
        fn name(&self) -> &str {
            &self.name
        }
    }

    impl Deref for Node {
        type Target = Base;

        fn deref(&self) -> &Self::Target {
            self.0.deref()
        }
    }

    impl DerefMut for Node {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.0.deref_mut()
        }
    }

    /// A wrapper for node pool record that allows to define custom visit method to have full
    /// control over instantiation process at deserialization.
    #[derive(Debug, Default, Clone, PartialEq, Reflect)]
    #[reflect(type_uuid = "08052dac-c0c0-4df3-8005-1330827bf9c2")]
    pub struct NodeContainer(Option<Node>);

    impl Visit for NodeContainer {
        fn visit(&mut self, _name: &str, _visitor: &mut Visitor) -> VisitResult {
            // Dummy impl.
            Ok(())
        }
    }

    impl PayloadContainer for NodeContainer {
        type Element = Node;

        fn new_empty() -> Self {
            Self(None)
        }

        fn new(element: Self::Element) -> Self {
            Self(Some(element))
        }

        fn is_some(&self) -> bool {
            self.0.is_some()
        }

        fn as_ref(&self) -> Option<&Self::Element> {
            self.0.as_ref()
        }

        fn as_mut(&mut self) -> Option<&mut Self::Element> {
            self.0.as_mut()
        }

        fn replace(&mut self, element: Self::Element) -> Option<Self::Element> {
            self.0.replace(element)
        }

        fn take(&mut self) -> Option<Self::Element> {
            self.0.take()
        }
    }

    impl NodeWrapper for Node {
        type Base = Base;
        type SceneGraph = Graph;
        type ResourceData = Graph;

        fn inner_ref(&self) -> &dyn Reflect {
            self.0.deref()
        }

        fn inner_mut(&mut self) -> &mut dyn Reflect {
            self.0.deref_mut()
        }

        #[allow(clippy::explicit_auto_deref)] // False-positive
        fn base(&self) -> &Self::Base {
            &**self
        }

        fn set_base(&mut self, base: Self::Base) {
            **self = base;
        }

        fn is_resource_instance_root(&self) -> bool {
            self.is_resource_instance_root
        }

        fn original_handle_in_resource(&self) -> Handle<Self> {
            self.original_handle_in_resource
        }

        fn set_original_handle_in_resource(&mut self, handle: Handle<Self>) {
            self.original_handle_in_resource = handle;
        }

        fn resource(&self) -> Option<Resource<Self::ResourceData>> {
            self.resource.clone()
        }

        fn self_handle(&self) -> Handle<Self> {
            self.self_handle
        }

        fn parent(&self) -> Handle<Self> {
            self.parent
        }

        fn children(&self) -> &[Handle<Self>] {
            &self.children
        }

        fn children_mut(&mut self) -> &mut [Handle<Self>] {
            &mut self.children
        }

        fn instance_id(&self) -> Uuid {
            self.instance_id
        }
    }

    #[derive(Default, Clone, Visit, Reflect, PartialEq, Debug)]
    #[reflect(type_uuid = "fc887063-7780-44af-8710-5e0bcf9a83fd")]
    pub struct Graph {
        root: Handle<Node>,
        nodes: Pool<Node, NodeContainer>,
    }

    impl ResourceData for Graph {
        fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn can_be_saved(&self) -> bool {
            false
        }

        fn try_clone_box(&self) -> Option<Box<dyn ResourceData>> {
            Some(Box::new(self.clone()))
        }
    }

    impl PrefabData for Graph {
        type Graph = Graph;

        fn graph(&self) -> &Self::Graph {
            self
        }

        fn mapping(&self) -> NodeMapping {
            NodeMapping::UseHandles
        }
    }

    impl Index<Handle<Node>> for Graph {
        type Output = Node;

        #[inline]
        fn index(&self, index: Handle<Node>) -> &Self::Output {
            &self.nodes[index]
        }
    }

    impl IndexMut<Handle<Node>> for Graph {
        #[inline]
        fn index_mut(&mut self, index: Handle<Node>) -> &mut Self::Output {
            &mut self.nodes[index]
        }
    }

    impl SceneGraph for Graph {
        type Prefab = Graph;
        type NodeWrapper = Node;

        fn summary(&self) -> String {
            "Summary".to_string()
        }

        fn root(&self) -> Handle<Self::NodeWrapper> {
            self.root
        }

        fn set_root(&mut self, root: Handle<Self::NodeWrapper>) {
            self.root = root;
        }

        fn is_valid_handle(&self, handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>) -> bool {
            self.nodes.is_valid_handle(handle)
        }

        fn add_node(&mut self, node: Self::NodeWrapper) -> Handle<Self::NodeWrapper> {
            let handle = self.nodes.next_free_handle();
            self.add_node_at_handle(node, handle);
            handle
        }

        fn add_node_at_handle(
            &mut self,
            mut node: Self::NodeWrapper,
            handle: Handle<Self::NodeWrapper>,
        ) {
            let children = node.children.clone();
            node.children.clear();
            let handle = self
                .nodes
                .spawn_at_handle(handle, node)
                .expect("The handle must be valid");

            if self.root.is_none() {
                self.root = handle;
            } else {
                self.link_nodes(handle, self.root);
            }

            for child in children {
                self.link_nodes(child, handle);
            }

            let node = &mut self.nodes[handle];
            node.self_handle = handle;
        }

        fn remove_node(&mut self, node_handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>) {
            let node_handle = node_handle.to_base();
            self.isolate_node(node_handle);
            let mut stack = vec![node_handle];
            while let Some(handle) = stack.pop() {
                stack.extend_from_slice(self.nodes[handle].children());
                self.nodes.free(handle);
            }
        }

        fn link_nodes(
            &mut self,
            child: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
            parent: Handle<impl ObjectOrVariant<Self::NodeWrapper>>,
        ) {
            let child = child.to_base();
            let parent = parent.to_base();
            self.isolate_node(child);
            self.nodes[child].parent = parent;
            self.nodes[parent].children.push(child);
        }

        fn unlink_node(&mut self, node_handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>) {
            self.isolate_node(node_handle);
            self.link_nodes(node_handle, self.root);
        }

        fn isolate_node(&mut self, node_handle: Handle<impl ObjectOrVariant<Self::NodeWrapper>>) {
            let node_handle = node_handle.to_base();

            let parent_handle =
                std::mem::replace(&mut self.nodes[node_handle].parent, Handle::NONE);

            if let Ok(parent) = self.nodes.try_borrow_mut(parent_handle) {
                if let Some(i) = parent.children().iter().position(|h| *h == node_handle) {
                    parent.children.remove(i);
                }
            }
        }

        fn try_get_node(
            &self,
            handle: Handle<Self::NodeWrapper>,
        ) -> Result<&Self::NodeWrapper, PoolError> {
            self.nodes.try_borrow(handle)
        }

        fn try_get_node_mut(
            &mut self,
            handle: Handle<Self::NodeWrapper>,
        ) -> Result<&mut Self::NodeWrapper, PoolError> {
            self.nodes.try_borrow_mut(handle)
        }

        fn actual_type_id(&self, handle: Handle<Self::NodeWrapper>) -> Result<TypeId, PoolError> {
            self.nodes
                .try_borrow(handle)
                .map(|n| NodeAsAny::as_any(n.0.deref()).type_id())
        }

        fn derived_type_ids(
            &self,
            handle: Handle<Self::NodeWrapper>,
        ) -> Result<Vec<TypeId>, PoolError> {
            self.nodes
                .try_borrow(handle)
                .map(|n| n.0.deref().type_info_ref().derived_types.to_vec())
        }

        fn actual_type_name(
            &self,
            handle: Handle<Self::NodeWrapper>,
        ) -> Result<&'static str, PoolError> {
            self.nodes
                .try_borrow(handle)
                .map(|n| n.0.deref().type_info_ref().type_name)
        }

        fn pair_iter(
            &self,
        ) -> impl Iterator<Item = (Handle<Self::NodeWrapper>, &Self::NodeWrapper)> {
            self.nodes.pair_iter()
        }

        fn linear_iter(&self) -> impl Iterator<Item = &Self::NodeWrapper> {
            self.nodes.iter()
        }

        fn linear_iter_mut(&mut self) -> impl Iterator<Item = &mut Self::NodeWrapper> {
            self.nodes.iter_mut()
        }

        fn try_get<U: ObjectOrVariant<Node>>(&self, handle: Handle<U>) -> Result<&U, PoolError> {
            self.nodes.try_get(handle)
        }

        fn try_get_mut<U: ObjectOrVariant<Node>>(
            &mut self,
            handle: Handle<U>,
        ) -> Result<&mut U, PoolError> {
            self.nodes.try_get_mut(handle)
        }
    }

    fn remap_handles(old_new_mapping: &NodeHandleMap<Node>, dest_graph: &mut Graph) {
        // Iterate over instantiated nodes and remap handles.
        for (_, &new_node_handle) in old_new_mapping.inner().iter() {
            old_new_mapping.remap_handles(
                &mut dest_graph.nodes[new_node_handle],
                &[TypeId::of::<UntypedResource>()],
            );
        }
    }

    fn clear_links(mut node: Node) -> Node {
        node.children.clear();
        node.parent = Handle::NONE;
        node
    }

    impl Graph {
        #[inline]
        pub fn copy_node(
            &self,
            node_handle: Handle<Node>,
            dest_graph: &mut Graph,
        ) -> (Handle<Node>, NodeHandleMap<Node>) {
            let mut old_new_mapping = NodeHandleMap::default();
            let root_handle = self.copy_node_raw(node_handle, dest_graph, &mut old_new_mapping);

            remap_handles(&old_new_mapping, dest_graph);

            (root_handle, old_new_mapping)
        }
        fn copy_node_raw(
            &self,
            root_handle: Handle<Node>,
            dest_graph: &mut Graph,
            old_new_mapping: &mut NodeHandleMap<Node>,
        ) -> Handle<Node> {
            let src_node = &self.nodes[root_handle];
            let dest_node = clear_links(src_node.clone());
            let dest_copy_handle = dest_graph.add_node(dest_node);
            old_new_mapping.insert(root_handle, dest_copy_handle);
            for &src_child_handle in src_node.children() {
                let dest_child_handle =
                    self.copy_node_raw(src_child_handle, dest_graph, old_new_mapping);
                if !dest_child_handle.is_none() {
                    dest_graph.link_nodes(dest_child_handle, dest_copy_handle);
                }
            }
            dest_copy_handle
        }
    }

    #[derive(Clone, Reflect, Visit, PartialEq, Default, Debug)]
    #[reflect(derived_type = "Node")]
    #[reflect(type_uuid = "c7452fb6-78c1-4e77-a2f9-d9ae31f50327")]
    pub struct Pivot {
        base: Base,
    }

    impl NodeTrait for Pivot {}

    impl Deref for Pivot {
        type Target = Base;

        fn deref(&self) -> &Self::Target {
            &self.base
        }
    }

    impl DerefMut for Pivot {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.base
        }
    }

    #[derive(Clone, Reflect, Visit, PartialEq, Default, Debug)]
    #[reflect(derived_type = "Node")]
    #[reflect(type_uuid = "17cc2d25-6ba4-4e2d-a31e-867e429bc659")]
    pub struct RigidBody {
        base: Base,
    }

    impl NodeTrait for RigidBody {}

    impl Deref for RigidBody {
        type Target = Base;

        fn deref(&self) -> &Self::Target {
            &self.base
        }
    }

    impl DerefMut for RigidBody {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.base
        }
    }

    #[derive(Clone, Reflect, Visit, PartialEq, Default, Debug)]
    #[reflect(derived_type = "Node")]
    #[reflect(type_uuid = "1f869298-37b1-4153-a2a9-6576daa0e8b3")]
    pub struct Joint {
        base: Base,
        connected_body1: Handle<RigidBody>,
        connected_body2: Handle<RigidBody>,
    }

    impl NodeTrait for Joint {}

    impl Deref for Joint {
        type Target = Base;

        fn deref(&self) -> &Self::Target {
            &self.base
        }
    }

    impl DerefMut for Joint {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.base
        }
    }

    #[test]
    fn test_set_child_position() {
        let mut graph = Graph::default();

        let root = graph.add_node(Node::new(Pivot::default()));
        let a = graph.add_node(Node::new(Pivot::default()));
        let b = graph.add_node(Node::new(Pivot::default()));
        let c = graph.add_node(Node::new(Pivot::default()));
        let d = graph.add_node(Node::new(Pivot::default()));
        graph.link_nodes(a, root);
        graph.link_nodes(b, root);
        graph.link_nodes(c, root);
        graph.link_nodes(d, root);

        assert!(graph.try_get_of_type::<Pivot>(a).is_ok());

        let root_ref = &mut graph[root];
        assert_eq!(root_ref.set_child_position(a, 0), Some(0));
        assert_eq!(root_ref.set_child_position(b, 1), Some(1));
        assert_eq!(root_ref.set_child_position(c, 2), Some(2));
        assert_eq!(root_ref.set_child_position(d, 3), Some(3));
        assert_eq!(root_ref.children[0], a);
        assert_eq!(root_ref.children[1], b);
        assert_eq!(root_ref.children[2], c);
        assert_eq!(root_ref.children[3], d);

        let initial_pos = root_ref.set_child_position(a, 3);
        assert_eq!(initial_pos, Some(0));
        assert_eq!(root_ref.children[0], b);
        assert_eq!(root_ref.children[1], c);
        assert_eq!(root_ref.children[2], d);
        assert_eq!(root_ref.children[3], a);

        let prev_pos = root_ref.set_child_position(a, initial_pos.unwrap());
        assert_eq!(prev_pos, Some(3));
        assert_eq!(root_ref.children[0], a);
        assert_eq!(root_ref.children[1], b);
        assert_eq!(root_ref.children[2], c);
        assert_eq!(root_ref.children[3], d);

        assert_eq!(root_ref.set_child_position(d, 1), Some(3));
        assert_eq!(root_ref.children[0], a);
        assert_eq!(root_ref.children[1], d);
        assert_eq!(root_ref.children[2], b);
        assert_eq!(root_ref.children[3], c);

        assert_eq!(root_ref.set_child_position(d, 0), Some(1));
        assert_eq!(root_ref.children[0], d);
        assert_eq!(root_ref.children[1], a);
        assert_eq!(root_ref.children[2], b);
        assert_eq!(root_ref.children[3], c);
    }

    #[test]
    fn test_derived_handles_mapping() {
        let mut prefab_graph = Graph::default();

        prefab_graph.add_node(Node::new(Pivot::default()));
        let rigid_body = prefab_graph.add_node(Node::new(RigidBody::default()));
        let rigid_body2 = prefab_graph.add_node(Node::new(RigidBody::default()));
        let joint = prefab_graph.add_node(Node::new(Joint {
            base: Base::default(),
            connected_body1: rigid_body.transmute(),
            connected_body2: rigid_body2.transmute(),
        }));

        let mut scene_graph = Graph::default();
        let root = scene_graph.add_node(Node::new(Pivot::default()));

        let (_, mapping) = prefab_graph.copy_node(root, &mut scene_graph);
        let rigid_body_copy = mapping
            .inner()
            .get(&rigid_body)
            .cloned()
            .unwrap()
            .transmute::<RigidBody>();
        let rigid_body2_copy = mapping
            .inner()
            .get(&rigid_body2)
            .cloned()
            .unwrap()
            .transmute::<RigidBody>();
        let joint_copy = mapping.inner().get(&joint).cloned().unwrap();
        let joint_copy_ref = scene_graph.nodes[joint_copy]
            .inner_ref()
            .downcast_ref::<Joint>()
            .unwrap();
        assert_eq!(joint_copy_ref.connected_body1, rigid_body_copy);
        assert_eq!(joint_copy_ref.connected_body2, rigid_body2_copy);
    }

    #[test]
    fn test_change_root() {
        let mut graph = Graph::default();

        // Root_
        //      |_A_
        //          |_B
        //          |_C_
        //             |_D
        let root = graph.add_node(Node::new(Pivot::default()));
        let d = graph.add_node(Node::new(Pivot::default()));
        let c = graph.add_node(Node::new(Pivot {
            base: Base {
                children: vec![d],
                ..Default::default()
            },
        }));
        let b = graph.add_node(Node::new(Pivot::default()));
        let a = graph.add_node(Node::new(Pivot {
            base: Base {
                children: vec![b, c],
                ..Default::default()
            },
        }));
        graph.link_nodes(a, root);

        dbg!(root, a, b, c, d);

        let link_scheme = graph.change_hierarchy_root(root, c);

        // C_
        //   |_D
        //   |_A_
        //       |_B
        //   |_Root
        assert_eq!(graph.root, c);

        assert_eq!(graph[graph.root].parent, Handle::<Node>::NONE);
        assert_eq!(graph[graph.root].children.len(), 3);

        assert_eq!(graph[graph.root].children[0], d);
        assert_eq!(graph[d].parent, graph.root);
        assert!(graph[d].children.is_empty());

        assert_eq!(graph[graph.root].children[1], a);
        assert_eq!(graph[a].parent, graph.root);

        assert_eq!(graph[graph.root].children[2], root);
        assert_eq!(graph[root].parent, graph.root);

        assert_eq!(graph[a].children.len(), 1);
        assert_eq!(graph[a].children[0], b);
        assert_eq!(graph[b].parent, a);

        assert!(graph[b].children.is_empty());

        // Revert
        graph.apply_link_scheme(link_scheme);

        assert_eq!(graph.root, root);
        assert_eq!(graph[graph.root].parent, Handle::<Node>::NONE);
        assert_eq!(graph[graph.root].children, vec![a]);

        assert_eq!(graph[a].parent, root);
        assert_eq!(graph[a].children, vec![b, c]);

        assert_eq!(graph[b].parent, a);
        assert_eq!(graph[b].children, Vec::<Handle<Node>>::new());

        assert_eq!(graph[c].parent, a);
        assert_eq!(graph[c].children, vec![d]);
    }

    #[test]
    fn test_traverse_iter() {
        let mut graph = Graph::default();

        // Root_
        //      |_A_
        //      |   |_B
        //      |   |_C_
        //      |      |_D_
        //      |         |_E
        //      |_F
        let root = graph.add_node(Node::new(Pivot::default()));
        let e = graph.add_node(Node::new(Pivot::default()));
        let d = graph.add_node(Node::new(Pivot {
            base: Base {
                children: vec![e],
                ..Default::default()
            },
        }));
        let c = graph.add_node(Node::new(Pivot {
            base: Base {
                children: vec![d],
                ..Default::default()
            },
        }));
        let b = graph.add_node(Node::new(Pivot::default()));
        let a = graph.add_node(Node::new(Pivot {
            base: Base {
                children: vec![b, c],
                ..Default::default()
            },
        }));
        let f = graph.add_node(Node::new(Pivot::default()));
        graph.link_nodes(a, root);
        graph.link_nodes(f, root);

        // test full depth traversal (immutable)
        let mut iter = graph.traverse_handle_iter(root);
        assert_eq!(iter.next(), Some(root));
        assert_eq!(iter.next(), Some(a));
        assert_eq!(iter.next(), Some(b));
        assert_eq!(iter.next(), Some(c));
        assert_eq!(iter.next(), Some(d));
        assert_eq!(iter.next(), Some(e));
        assert_eq!(iter.next(), Some(f));
        assert_eq!(iter.next(), None);

        drop(iter);

        // test sub-graph traversal (immutable)
        let mut iter = graph.traverse_handle_iter(d);
        assert_eq!(iter.next(), Some(d));
        assert_eq!(iter.next(), Some(e));
        assert_eq!(iter.next(), None);

        drop(iter);

        // test full depth traversal (mutable)
        let mut iter_mut = graph.traverse_iter_mut(root).map(|(h, _)| h);
        assert_eq!(iter_mut.next(), Some(root));
        assert_eq!(iter_mut.next(), Some(a));
        assert_eq!(iter_mut.next(), Some(b));
        assert_eq!(iter_mut.next(), Some(c));
        assert_eq!(iter_mut.next(), Some(d));
        assert_eq!(iter_mut.next(), Some(e));
        assert_eq!(iter_mut.next(), Some(f));
        assert_eq!(iter_mut.next(), None);

        drop(iter_mut);

        // test sub-graph traversal (mutable)
        let mut iter_mut = graph.traverse_iter_mut(d).map(|(h, _)| h);
        assert_eq!(iter_mut.next(), Some(d));
        assert_eq!(iter_mut.next(), Some(e));
        assert_eq!(iter_mut.next(), None);
    }
}
