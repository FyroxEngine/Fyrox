//! Graph utilities and common algorithms.

use fxhash::FxHashMap;
use fyrox_core::{
    log::{Log, MessageKind},
    pool::Handle,
    reflect::prelude::*,
    variable::{self, InheritableVariable},
    NameProvider,
};
use fyrox_resource::{untyped::UntypedResource, Resource, TypedResourceData};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Reflect)]
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

pub trait SceneGraphNode: Reflect + NameProvider + Sized + Clone + 'static {
    type Base: Clone;
    type ResourceData: TypedResourceData;

    fn base(&self) -> &Self::Base;
    fn set_base(&mut self, base: Self::Base);
    fn is_resource_instance_root(&self) -> bool;
    fn original_handle_in_resource(&self) -> Handle<Self>;
    fn set_original_handle_in_resource(&mut self, handle: Handle<Self>);
    fn resource(&self) -> Option<Resource<Self::ResourceData>>;
    fn self_handle(&self) -> Handle<Self>;
    fn parent(&self) -> Handle<Self>;
    fn children(&self) -> &[Handle<Self>];
}

pub trait PrefabData: TypedResourceData + 'static {
    type Graph: SceneGraph;

    fn graph(&self) -> &Self::Graph;
    fn mapping(&self) -> NodeMapping;
}

pub trait SceneGraph: Sized + 'static {
    type Prefab: PrefabData<Graph = Self>;
    type Node: SceneGraphNode<ResourceData = Self::Prefab>;

    /// Returns a handle of the root node of the graph.
    fn root(&self) -> Handle<Self::Node>;

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    fn pair_iter(&self) -> impl Iterator<Item = (Handle<Self::Node>, &Self::Node)>;

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    fn linear_iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Node>;

    /// Checks whether the given node handle is valid or not.
    fn is_valid_handle(&self, handle: Handle<Self::Node>) -> bool;

    /// Destroys the node and its children recursively.
    fn remove_node(&mut self, node_handle: Handle<Self::Node>);

    /// Links specified child with specified parent.
    fn link_nodes(&mut self, child: Handle<Self::Node>, parent: Handle<Self::Node>);

    /// Borrows a node by its handle.
    fn node(&self, handle: Handle<Self::Node>) -> &Self::Node;

    /// Tries to borrow a node, returns Some(node) if the handle is valid, None - otherwise.
    fn try_get(&self, handle: Handle<Self::Node>) -> Option<&Self::Node>;

    /// Create a graph depth traversal iterator.
    fn traverse_iter(&self, from: Handle<Self::Node>) -> impl Iterator<Item = &Self::Node> {
        GraphTraverseIterator {
            graph: self,
            stack: vec![from],
        }
    }

    /// Create a graph depth traversal iterator.
    fn traverse_handle_iter(
        &self,
        from: Handle<Self::Node>,
    ) -> impl Iterator<Item = Handle<Self::Node>> {
        GraphHandleTraverseIterator {
            graph: self,
            stack: vec![from],
        }
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure.
    /// Returns a tuple with a handle and a reference to the found node. If nothing is found, it
    /// returns [`None`].
    fn find<C>(
        &self,
        root_node: Handle<Self::Node>,
        cmp: &mut C,
    ) -> Option<(Handle<Self::Node>, &Self::Node)>
    where
        C: FnMut(&Self::Node) -> bool,
    {
        self.try_get(root_node).and_then(|root| {
            if cmp(root) {
                Some((root_node, root))
            } else {
                root.children().iter().find_map(|c| self.find(*c, cmp))
            }
        })
    }

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

    fn restore_original_handles_and_inherit_properties<F>(
        &mut self,
        ignored_types: &[TypeId],
        mut before_inherit: F,
    ) where
        F: FnMut(&Self::Node, &mut Self::Node),
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
                let model_kind = header.kind().clone();
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
                                .try_get(node.original_handle_in_resource())
                                .map(|resource_node| {
                                    (resource_node, node.original_handle_in_resource())
                                })
                        }
                    };

                    if let Some((resource_node, original)) = resource_node {
                        node.set_original_handle_in_resource(original);

                        before_inherit(resource_node, node);

                        // Check if the actual node types (this and parent's) are equal, and if not - copy the
                        // node and replace its base.
                        let mut types_match = true;
                        node.as_reflect(&mut |node_reflect| {
                            resource_node.as_reflect(&mut |resource_node_reflect| {
                                types_match =
                                    node_reflect.type_id() == resource_node_reflect.type_id();

                                if !types_match {
                                    Log::warn(format!(
                                        "Node {}({}:{}) instance \
                                        have different type than in the respective parent \
                                        asset {}. The type will be fixed.",
                                        node.name(),
                                        node.self_handle().index(),
                                        node.self_handle().generation(),
                                        model_kind
                                    ));
                                }
                            })
                        });
                        if !types_match {
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
                        node.as_reflect_mut(&mut |node_reflect| {
                            resource_node.as_reflect(&mut |resource_node_reflect| {
                                Log::verify(variable::try_inherit_properties(
                                    node_reflect,
                                    resource_node_reflect,
                                    &ignored_types,
                                ));
                            })
                        })
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
}

/// Iterator that traverses tree in depth and returns shared references to nodes.
pub struct GraphTraverseIterator<'a, G, N> {
    graph: &'a G,
    stack: Vec<Handle<N>>,
}

impl<'a, G, N> Iterator for GraphTraverseIterator<'a, G, N>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode,
{
    type Item = &'a N;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(handle) = self.stack.pop() {
            let node = self.graph.node(handle);

            for child_handle in node.children() {
                self.stack.push(*child_handle);
            }

            return Some(node);
        }

        None
    }
}

/// Iterator that traverses tree in depth and returns handles to nodes.
pub struct GraphHandleTraverseIterator<'a, G, N> {
    graph: &'a G,
    stack: Vec<Handle<N>>,
}

impl<'a, G, N> Iterator for GraphHandleTraverseIterator<'a, G, N>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode,
{
    type Item = Handle<N>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(handle) = self.stack.pop() {
            for child_handle in self.graph.node(handle).children() {
                self.stack.push(*child_handle);
            }

            return Some(handle);
        }
        None
    }
}
