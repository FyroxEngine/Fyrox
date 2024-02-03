//! Prefab utilities.

use fxhash::FxHashMap;
use fyrox_core::{
    log::{Log, MessageKind},
    pool::Handle,
    reflect::prelude::*,
    variable::InheritableVariable,
    NameProvider,
};
use fyrox_resource::{Resource, TypedResourceData};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

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
    pub fn insert(
        &mut self,
        original_handle: Handle<N>,
        copy_handle: Handle<N>,
    ) -> Option<Handle<N>> {
        self.map.insert(original_handle, copy_handle)
    }

    /// Maps a handle to a handle of its origin, or sets it to [Handle::NONE] if there is no such node.
    /// It should be used when you are sure that respective origin exists.
    pub fn map(&self, handle: &mut Handle<N>) -> &Self {
        *handle = self.map.get(handle).cloned().unwrap_or_default();
        self
    }

    /// Maps each handle in the slice to a handle of its origin, or sets it to [Handle::NONE] if there is no such node.
    /// It should be used when you are sure that respective origin exists.
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
    pub fn try_map(&self, handle: &mut Handle<N>) -> bool {
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
    pub fn try_map_silent(&self, inheritable_handle: &mut InheritableVariable<Handle<N>>) -> bool {
        if let Some(new_handle) = self.map.get(inheritable_handle) {
            inheritable_handle.set_value_silent(*new_handle);
            true
        } else {
            false
        }
    }

    /// Returns a shared reference to inner map.
    pub fn inner(&self) -> &FxHashMap<Handle<N>, Handle<N>> {
        &self.map
    }

    /// Returns inner map.
    pub fn into_inner(self) -> FxHashMap<Handle<N>, Handle<N>> {
        self.map
    }

    /// Tries to remap handles to nodes in a given entity using reflection. It finds all supported fields recursively
    /// (`Handle<Node>`, `Vec<Handle<Node>>`, `InheritableVariable<Handle<Node>>`, `InheritableVariable<Vec<Handle<Node>>>`)
    /// and automatically maps old handles to new.
    pub fn remap_handles(&self, node: &mut N, ignored_types: &[TypeId]) {
        let name = node.name().to_string();
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

        entity.downcast_mut::<Handle<N>>(&mut |handle| {
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

        entity.downcast_mut::<Vec<Handle<N>>>(&mut |vec| {
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

    pub fn remap_inheritable_handles(&self, node: &mut N, ignored_types: &[TypeId]) {
        let name = node.name().to_string();
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

        entity.downcast_mut::<Handle<N>>(&mut |result| {
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

        entity.downcast_mut::<Vec<Handle<N>>>(&mut |result| {
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
                    *field,
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

pub trait SceneGraphNode: Reflect + NameProvider + Sized + 'static {
    type ResourceData: TypedResourceData;

    fn is_resource_instance_root(&self) -> bool;
    fn original_handle_in_resource(&self) -> Handle<Self>;
    fn resource(&self) -> Option<Resource<Self::ResourceData>>;
    fn self_handle(&self) -> Handle<Self>;
    fn parent(&self) -> Handle<Self>;
    fn children(&self) -> &[Handle<Self>];
}

pub trait PrefabData: TypedResourceData + 'static {
    type Graph: SceneGraph;

    fn graph(&self) -> &Self::Graph;
}

pub trait SceneGraph: Sized + 'static {
    type Prefab: PrefabData<Graph = Self>;
    type Node: SceneGraphNode<ResourceData = Self::Prefab>;

    /// Returns a handle of the root node of the graph.
    fn root(&self) -> Handle<Self::Node>;

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    fn pair_iter(&self) -> impl Iterator<Item = (Handle<Self::Node>, &Self::Node)>;

    /// Create a graph depth traversal iterator.
    fn traverse_iter(&self, from: Handle<Self::Node>) -> impl Iterator<Item = &Self::Node>;

    /// Checks whether the given node handle is valid or not.
    fn is_valid_handle(&self, handle: Handle<Self::Node>) -> bool;

    /// Destroys the node and its children recursively.
    fn remove_node(&mut self, node_handle: Handle<Self::Node>);

    /// Links specified child with specified parent.
    fn link_nodes(&mut self, child: Handle<Self::Node>, parent: Handle<Self::Node>);

    /// Borrows a node by its handle.
    fn node(&self, handle: Handle<Self::Node>) -> &Self::Node;

    /// Searches for a node down the tree starting from the specified node using the specified closure.
    /// Returns a tuple with a handle and a reference to the found node. If nothing is found, it
    /// returns [`None`].
    fn find<C>(
        &self,
        root_node: Handle<Self::Node>,
        cmp: &mut C,
    ) -> Option<(Handle<Self::Node>, &Self::Node)>
    where
        C: FnMut(&Self::Node) -> bool;

    /// This method checks integrity of the graph and restores it if needed. For example, if a node
    /// was added in a parent asset, then it must be added in the graph. Alternatively, if a node was
    /// deleted in a parent asset, then its instance must be deleted in the graph.
    #[allow(clippy::type_complexity)]
    fn restore_integrity<F>(
        &mut self,
        mut instantiate: F,
    ) -> Vec<(Handle<Self::Node>, Resource<Self::Prefab>)>
    where
        F: FnMut(
            Resource<Self::Prefab>,
            &Self::Prefab,
            Handle<Self::Node>,
            &mut Self,
        ) -> (Handle<Self::Node>, NodeHandleMap<Self::Node>),
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
            for node in self.traverse_iter(instance_root) {
                if let Some(resource) = node.resource() {
                    let kind = resource.kind().clone();
                    if let Some(model) = resource.state().data() {
                        if !model
                            .graph()
                            .is_valid_handle(node.original_handle_in_resource())
                        {
                            nodes_to_delete.push(node.self_handle());

                            Log::warn(format!(
                                "Node {} ({}:{}) and its children will be deleted, because it \
                    does not exist in the parent asset `{}`!",
                                node.name(),
                                node.self_handle().index(),
                                node.self_handle().generation(),
                                kind
                            ))
                        }
                    } else {
                        Log::warn(format!(
                            "Node {} ({}:{}) and its children will be deleted, because its \
                    parent asset `{}` failed to load!",
                            node.name(),
                            node.self_handle().index(),
                            node.self_handle().generation(),
                            kind
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
            let model_kind = model.kind().clone();
            if let Some(data) = model.data() {
                let resource_graph = data.graph();

                let resource_instance_root = self.node(instance_root).original_handle_in_resource();

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
                    let mut compare =
                        |n: &Self::Node| n.original_handle_in_resource() == resource_node_handle;

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
                "Integrity restored for {} instances! {} new nodes were added!",
                instance_count, restored_count
            ),
        );

        instances
    }
}
