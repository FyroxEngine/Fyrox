#![allow(clippy::type_complexity)]

//! Graph utilities and common algorithms.

use fxhash::FxHashMap;
use fyrox_core::pool::ErasedHandle;
use fyrox_core::{
    log::{Log, MessageKind},
    pool::Handle,
    reflect::prelude::*,
    variable::{self, InheritableVariable},
    ComponentProvider, NameProvider,
};
use fyrox_resource::{untyped::UntypedResource, Resource, TypedResourceData};
use std::any::Any;
use std::cmp::Ordering;
use std::fmt::Debug;
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

    #[inline]
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

fn reset_property_modified_flag(entity: &mut dyn Reflect, path: &str) {
    entity.as_reflect_mut(&mut |entity| {
        entity.resolve_path_mut(path, &mut |result| {
            variable::mark_inheritable_properties_non_modified(
                result.unwrap(),
                &[TypeId::of::<UntypedResource>()],
            );
        })
    })
}

pub trait AbstractSceneNode: ComponentProvider + Reflect + NameProvider {}

impl<T: SceneGraphNode> AbstractSceneNode for T {}

pub trait SceneGraphNode: AbstractSceneNode + Clone + 'static {
    type Base: Clone;
    type SceneGraph: SceneGraph<Node = Self>;
    type ResourceData: PrefabData<Graph = Self::SceneGraph>;

    fn base(&self) -> &Self::Base;
    fn set_base(&mut self, base: Self::Base);
    fn is_resource_instance_root(&self) -> bool;
    fn original_handle_in_resource(&self) -> Handle<Self>;
    fn set_original_handle_in_resource(&mut self, handle: Handle<Self>);
    fn resource(&self) -> Option<Resource<Self::ResourceData>>;
    fn self_handle(&self) -> Handle<Self>;
    fn parent(&self) -> Handle<Self>;
    fn children(&self) -> &[Handle<Self>];
    fn children_mut(&mut self) -> &mut [Handle<Self>];

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
            parent.as_reflect(&mut |parent| {
                parent.resolve_path(path, &mut |result| match result {
                    Ok(parent_field) => parent_field.as_inheritable_variable(&mut |parent_field| {
                        if let Some(parent_inheritable) = parent_field {
                            parent_value = Some(parent_inheritable.clone_value_box());
                        }
                    }),
                    Err(e) => Log::err(format!(
                        "Failed to resolve parent path {}. Reason: {:?}",
                        path, e
                    )),
                })
            });

            // Check whether the child's field is inheritable and modified.
            let mut need_revert = false;

            self.as_reflect_mut(&mut |child| {
                child.resolve_path_mut(path, &mut |result| match result {
                    Ok(child_field) => {
                        child_field.as_inheritable_variable_mut(&mut |child_inheritable| {
                            if let Some(child_inheritable) = child_inheritable {
                                need_revert = child_inheritable.is_modified();
                            } else {
                                Log::err(format!("Property {} is not inheritable!", path))
                            }
                        })
                    }
                    Err(e) => Log::err(format!(
                        "Failed to resolve child path {}. Reason: {:?}",
                        path, e
                    )),
                });
            });

            // Try to apply it to the child.
            if need_revert {
                if let Some(parent_value) = parent_value {
                    let mut was_set = false;

                    let mut parent_value = Some(parent_value);
                    self.as_reflect_mut(&mut |child| {
                        child.set_field_by_path(
                            path,
                            parent_value.take().unwrap(),
                            &mut |result| match result {
                                Ok(old_value) => {
                                    previous_value = Some(old_value);

                                    was_set = true;
                                }
                                Err(_) => Log::err(format!(
                                    "Failed to revert property {}. Reason: no such property!",
                                    path
                                )),
                            },
                        );
                    });

                    if was_set {
                        // Reset modified flag.
                        reset_property_modified_flag(self, path);
                    }
                }
            }
        }

        previous_value
    }

    /// Tries to borrow a component of given type.
    #[inline]
    fn component_ref<T: Any>(&self) -> Option<&T> {
        ComponentProvider::query_component_ref(self, TypeId::of::<T>())
            .and_then(|c| c.downcast_ref())
    }

    /// Tries to borrow a component of given type.
    #[inline]
    fn component_mut<T: Any>(&mut self) -> Option<&mut T> {
        ComponentProvider::query_component_mut(self, TypeId::of::<T>())
            .and_then(|c| c.downcast_mut())
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

pub trait AbstractSceneGraph: 'static {
    fn try_get_node_untyped(&self, handle: ErasedHandle) -> Option<&dyn AbstractSceneNode>;
    fn try_get_node_untyped_mut(
        &mut self,
        handle: ErasedHandle,
    ) -> Option<&mut dyn AbstractSceneNode>;
}

pub trait BaseSceneGraph: AbstractSceneGraph {
    type Prefab: PrefabData<Graph = Self>;
    type Node: SceneGraphNode<SceneGraph = Self, ResourceData = Self::Prefab>;

    /// Returns a handle of the root node of the graph.
    fn root(&self) -> Handle<Self::Node>;

    /// Sets the new root of the graph. If used incorrectly, it may create isolated sub-graphs.
    fn set_root(&mut self, root: Handle<Self::Node>);

    /// Tries to borrow a node, returns Some(node) if the handle is valid, None - otherwise.
    fn try_get(&self, handle: Handle<Self::Node>) -> Option<&Self::Node>;

    /// Tries to borrow a node, returns Some(node) if the handle is valid, None - otherwise.
    fn try_get_mut(&mut self, handle: Handle<Self::Node>) -> Option<&mut Self::Node>;

    /// Checks whether the given node handle is valid or not.
    fn is_valid_handle(&self, handle: Handle<Self::Node>) -> bool;

    /// Adds a new node to the graph.
    fn add_node(&mut self, node: Self::Node) -> Handle<Self::Node>;

    /// Destroys the node and its children recursively.
    fn remove_node(&mut self, node_handle: Handle<Self::Node>);

    /// Links specified child with specified parent.
    fn link_nodes(&mut self, child: Handle<Self::Node>, parent: Handle<Self::Node>);

    /// Unlinks specified node from its parent and attaches it to root graph node.
    fn unlink_node(&mut self, node_handle: Handle<Self::Node>);

    /// Detaches the node from its parent, making the node unreachable from any other node in the
    /// graph.
    fn isolate_node(&mut self, node_handle: Handle<Self::Node>);

    /// Borrows a node by its handle.
    fn node(&self, handle: Handle<Self::Node>) -> &Self::Node {
        self.try_get(handle).expect("The handle must be valid!")
    }

    /// Borrows a node by its handle.
    fn node_mut(&mut self, handle: Handle<Self::Node>) -> &mut Self::Node {
        self.try_get_mut(handle).expect("The handle must be valid!")
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
        prev_root: Handle<Self::Node>,
        new_root: Handle<Self::Node>,
    ) -> LinkScheme<Self::Node> {
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
    fn apply_link_scheme(&mut self, mut scheme: LinkScheme<Self::Node>) {
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
    fn remove_nodes(&mut self, nodes: &[Handle<Self::Node>]) {
        for &node in nodes {
            if self.is_valid_handle(node) {
                self.remove_node(node)
            }
        }
    }
}

pub trait SceneGraph: BaseSceneGraph {
    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    fn pair_iter(&self) -> impl Iterator<Item = (Handle<Self::Node>, &Self::Node)>;

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    fn linear_iter(&self) -> impl Iterator<Item = &Self::Node>;

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    fn linear_iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Node>;

    /// Tries to borrow a node and fetch its component of specified type.
    #[inline]
    fn try_get_of_type<T>(&self, handle: Handle<Self::Node>) -> Option<&T>
    where
        T: 'static,
    {
        self.try_get(handle)
            .and_then(|n| n.query_component_ref(TypeId::of::<T>()))
            .and_then(|c| c.downcast_ref())
    }

    /// Tries to mutably borrow a node and fetch its component of specified type.
    #[inline]
    fn try_get_mut_of_type<T>(&mut self, handle: Handle<Self::Node>) -> Option<&mut T>
    where
        T: 'static,
    {
        self.try_get_mut(handle)
            .and_then(|n| n.query_component_mut(TypeId::of::<T>()))
            .and_then(|c| c.downcast_mut())
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    fn find_map<C, T>(
        &self,
        root_node: Handle<Self::Node>,
        cmp: &mut C,
    ) -> Option<(Handle<Self::Node>, &T)>
    where
        C: FnMut(&Self::Node) -> Option<&T>,
        T: ?Sized,
    {
        self.try_get(root_node).and_then(|root| {
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
        root_node: Handle<Self::Node>,
        cmp: &mut C,
    ) -> Option<(Handle<Self::Node>, &Self::Node)>
    where
        C: FnMut(&Self::Node) -> bool,
    {
        let mut handle = root_node;
        while let Some(node) = self.try_get(handle) {
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
    fn find_handle_up<C>(&self, root_node: Handle<Self::Node>, cmp: &mut C) -> Handle<Self::Node>
    where
        C: FnMut(&Self::Node) -> bool,
    {
        self.find_up(root_node, cmp)
            .map(|(h, _)| h)
            .unwrap_or_default()
    }

    #[inline]
    fn find_component_up<T>(
        &self,
        node_handle: Handle<Self::Node>,
    ) -> Option<(Handle<Self::Node>, &T)>
    where
        T: 'static,
    {
        self.find_up_map(node_handle, &mut |node| {
            node.query_component_ref(TypeId::of::<T>())
        })
        .and_then(|(handle, c)| c.downcast_ref::<T>().map(|typed| (handle, typed)))
    }

    #[inline]
    fn find_component<T>(&self, node_handle: Handle<Self::Node>) -> Option<(Handle<Self::Node>, &T)>
    where
        T: 'static,
    {
        self.find_map(node_handle, &mut |node| {
            node.query_component_ref(TypeId::of::<T>())
        })
        .and_then(|(handle, c)| c.downcast_ref::<T>().map(|typed| (handle, typed)))
    }

    /// Searches for a node up the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    fn find_up_map<C, T>(
        &self,
        root_node: Handle<Self::Node>,
        cmp: &mut C,
    ) -> Option<(Handle<Self::Node>, &T)>
    where
        C: FnMut(&Self::Node) -> Option<&T>,
        T: ?Sized,
    {
        let mut handle = root_node;
        while let Some(node) = self.try_get(handle) {
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
        root_node: Handle<Self::Node>,
        name: &str,
    ) -> Option<(Handle<Self::Node>, &Self::Node)> {
        self.find(root_node, &mut |node| node.name() == name)
    }

    /// Searches for a node with the specified name up the tree starting from the specified node. Returns a tuple with a
    /// handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    fn find_up_by_name(
        &self,
        root_node: Handle<Self::Node>,
        name: &str,
    ) -> Option<(Handle<Self::Node>, &Self::Node)> {
        self.find_up(root_node, &mut |node| node.name() == name)
    }

    /// Searches for a node with the specified name down the tree starting from the graph root. Returns a tuple with a
    /// handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    fn find_by_name_from_root(&self, name: &str) -> Option<(Handle<Self::Node>, &Self::Node)> {
        self.find_by_name(self.root(), name)
    }

    #[inline]
    fn find_handle_by_name_from_root(&self, name: &str) -> Handle<Self::Node> {
        self.find_by_name(self.root(), name)
            .map(|(h, _)| h)
            .unwrap_or_default()
    }

    /// Searches node using specified compare closure starting from root. Returns a tuple with a handle and
    /// a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    fn find_from_root<C>(&self, cmp: &mut C) -> Option<(Handle<Self::Node>, &Self::Node)>
    where
        C: FnMut(&Self::Node) -> bool,
    {
        self.find(self.root(), cmp)
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure.
    /// Returns a tuple with a handle and a reference to the found node. If nothing is found, it
    /// returns [`None`].
    #[inline]
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

    /// The same as [`Self::find`], but only returns node handle which will be [`Handle::NONE`]
    /// if nothing is found.
    #[inline]
    fn find_handle<C>(&self, root_node: Handle<Self::Node>, cmp: &mut C) -> Handle<Self::Node>
    where
        C: FnMut(&Self::Node) -> bool,
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
        child: Handle<Self::Node>,
        offset: isize,
    ) -> Option<(Handle<Self::Node>, usize)> {
        let parents_parent_handle = self.try_get(child)?.parent();
        let parents_parent_ref = self.try_get(parents_parent_handle)?;
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
        from: Handle<Self::Node>,
    ) -> GraphTraverseIterator<'_, Self, Self::Node> {
        GraphTraverseIterator {
            graph: self,
            stack: vec![from],
        }
    }

    /// Create a graph depth traversal iterator.
    #[inline]
    fn traverse_handle_iter(
        &self,
        from: Handle<Self::Node>,
    ) -> GraphHandleTraverseIterator<'_, Self, Self::Node> {
        GraphHandleTraverseIterator {
            graph: self,
            stack: vec![from],
        }
    }

    /// This method checks integrity of the graph and restores it if needed. For example, if a node
    /// was added in a parent asset, then it must be added in the graph. Alternatively, if a node was
    /// deleted in a parent asset, then its instance must be deleted in the graph.
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

    /// Maps handles in properties of instances after property inheritance. It is needed, because when a
    /// property contains node handle, the handle cannot be used directly after inheritance. Instead, it
    /// must be mapped to respective instance first.
    ///
    /// To do so, we at first, build node handle mapping (original handle -> instance handle) starting from
    /// instance root. Then we must find all inheritable properties and try to remap them to instance handles.
    fn remap_handles(&mut self, instances: &[(Handle<Self::Node>, Resource<Self::Prefab>)]) {
        for (instance_root, resource) in instances {
            // Prepare old -> new handle mapping first by walking over the graph
            // starting from instance root.
            let mut old_new_mapping = NodeHandleMap::default();
            let mut traverse_stack = vec![*instance_root];
            while let Some(node_handle) = traverse_stack.pop() {
                let node = self.node(node_handle);
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
pub struct GraphTraverseIterator<'a, G: ?Sized, N> {
    graph: &'a G,
    stack: Vec<Handle<N>>,
}

impl<'a, G: ?Sized, N> Iterator for GraphTraverseIterator<'a, G, N>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode,
{
    type Item = &'a N;

    #[inline]
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
pub struct GraphHandleTraverseIterator<'a, G: ?Sized, N> {
    graph: &'a G,
    stack: Vec<Handle<N>>,
}

impl<'a, G, N> Iterator for GraphHandleTraverseIterator<'a, G, N>
where
    G: SceneGraph<Node = N>,
    N: SceneGraphNode,
{
    type Item = Handle<N>;

    #[inline]
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

#[cfg(test)]
mod test {
    use crate::{
        AbstractSceneGraph, AbstractSceneNode, BaseSceneGraph, NodeMapping, PrefabData, SceneGraph,
        SceneGraphNode,
    };
    use fyrox_core::pool::ErasedHandle;
    use fyrox_core::{
        pool::{Handle, Pool},
        reflect::prelude::*,
        type_traits::prelude::*,
        visitor::prelude::*,
        NameProvider,
    };
    use fyrox_resource::{Resource, ResourceData};
    use std::{
        any::Any,
        error::Error,
        ops::{Deref, DerefMut, Index, IndexMut},
        path::Path,
    };

    #[derive(Default, Visit, Reflect, Debug, Clone)]
    struct Base {
        name: String,
        self_handle: Handle<Node>,
        is_resource_instance_root: bool,
        original_handle_in_resource: Handle<Node>,
        resource: Option<Resource<Graph>>,
        parent: Handle<Node>,
        children: Vec<Handle<Node>>,
    }

    #[derive(Clone, ComponentProvider, Visit, Reflect, Debug, Default)]
    struct Node {
        base: Base,
    }

    impl NameProvider for Node {
        fn name(&self) -> &str {
            &self.base.name
        }
    }

    impl Deref for Node {
        type Target = Base;

        fn deref(&self) -> &Self::Target {
            &self.base
        }
    }

    impl DerefMut for Node {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.base
        }
    }

    impl SceneGraphNode for Node {
        type Base = Base;
        type SceneGraph = Graph;
        type ResourceData = Graph;

        fn base(&self) -> &Self::Base {
            &self.base
        }

        fn set_base(&mut self, base: Self::Base) {
            self.base = base;
        }

        fn is_resource_instance_root(&self) -> bool {
            self.base.is_resource_instance_root
        }

        fn original_handle_in_resource(&self) -> Handle<Self> {
            self.base.original_handle_in_resource
        }

        fn set_original_handle_in_resource(&mut self, handle: Handle<Self>) {
            self.base.original_handle_in_resource = handle;
        }

        fn resource(&self) -> Option<Resource<Self::ResourceData>> {
            self.base.resource.clone()
        }

        fn self_handle(&self) -> Handle<Self> {
            self.base.self_handle
        }

        fn parent(&self) -> Handle<Self> {
            self.base.parent
        }

        fn children(&self) -> &[Handle<Self>] {
            &self.base.children
        }

        fn children_mut(&mut self) -> &mut [Handle<Self>] {
            &mut self.base.children
        }
    }

    #[derive(Default, TypeUuidProvider, Visit, Reflect, Debug)]
    #[type_uuid(id = "fc887063-7780-44af-8710-5e0bcf9a83fd")]
    struct Graph {
        root: Handle<Node>,
        nodes: Pool<Node>,
    }

    impl ResourceData for Graph {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        fn type_uuid(&self) -> Uuid {
            <Graph as TypeUuidProvider>::type_uuid()
        }

        fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn can_be_saved(&self) -> bool {
            false
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

    impl AbstractSceneGraph for Graph {
        fn try_get_node_untyped(&self, handle: ErasedHandle) -> Option<&dyn AbstractSceneNode> {
            self.nodes
                .try_borrow(handle.into())
                .map(|n| n as &dyn AbstractSceneNode)
        }

        fn try_get_node_untyped_mut(
            &mut self,
            handle: ErasedHandle,
        ) -> Option<&mut dyn AbstractSceneNode> {
            self.nodes
                .try_borrow_mut(handle.into())
                .map(|n| n as &mut dyn AbstractSceneNode)
        }
    }

    impl BaseSceneGraph for Graph {
        type Prefab = Graph;
        type Node = Node;

        fn root(&self) -> Handle<Self::Node> {
            self.root
        }

        fn set_root(&mut self, root: Handle<Self::Node>) {
            self.root = root;
        }

        fn is_valid_handle(&self, handle: Handle<Self::Node>) -> bool {
            self.nodes.is_valid_handle(handle)
        }

        fn add_node(&mut self, mut node: Self::Node) -> Handle<Self::Node> {
            let children = node.base.children.clone();
            node.base.children.clear();
            let handle = self.nodes.spawn(node);

            if self.root.is_none() {
                self.root = handle;
            } else {
                self.link_nodes(handle, self.root);
            }

            for child in children {
                self.link_nodes(child, handle);
            }

            let node = &mut self.nodes[handle];
            node.base.self_handle = handle;
            handle
        }

        fn remove_node(&mut self, node_handle: Handle<Self::Node>) {
            self.isolate_node(node_handle);
            let mut stack = vec![node_handle];
            while let Some(handle) = stack.pop() {
                stack.extend_from_slice(self.nodes[handle].children());
                self.nodes.free(handle);
            }
        }

        fn link_nodes(&mut self, child: Handle<Self::Node>, parent: Handle<Self::Node>) {
            self.isolate_node(child);
            self.nodes[child].base.parent = parent;
            self.nodes[parent].base.children.push(child);
        }

        fn unlink_node(&mut self, node_handle: Handle<Self::Node>) {
            self.isolate_node(node_handle);
            self.link_nodes(node_handle, self.root);
        }

        fn isolate_node(&mut self, node_handle: Handle<Self::Node>) {
            let parent_handle =
                std::mem::replace(&mut self.nodes[node_handle].base.parent, Handle::NONE);

            if let Some(parent) = self.nodes.try_borrow_mut(parent_handle) {
                if let Some(i) = parent.children().iter().position(|h| *h == node_handle) {
                    parent.base.children.remove(i);
                }
            }
        }

        fn try_get(&self, handle: Handle<Self::Node>) -> Option<&Self::Node> {
            self.nodes.try_borrow(handle)
        }

        fn try_get_mut(&mut self, handle: Handle<Self::Node>) -> Option<&mut Self::Node> {
            self.nodes.try_borrow_mut(handle)
        }
    }

    impl SceneGraph for Graph {
        fn pair_iter(&self) -> impl Iterator<Item = (Handle<Self::Node>, &Self::Node)> {
            self.nodes.pair_iter()
        }

        fn linear_iter(&self) -> impl Iterator<Item = &Self::Node> {
            self.nodes.iter()
        }

        fn linear_iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Node> {
            self.nodes.iter_mut()
        }
    }

    #[test]
    fn test_set_child_position() {
        let mut graph = Graph::default();

        let root = graph.add_node(Node::default());
        let a = graph.add_node(Node::default());
        let b = graph.add_node(Node::default());
        let c = graph.add_node(Node::default());
        let d = graph.add_node(Node::default());
        graph.link_nodes(a, root);
        graph.link_nodes(b, root);
        graph.link_nodes(c, root);
        graph.link_nodes(d, root);

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
    fn test_change_root() {
        let mut graph = Graph::default();

        // Root_
        //      |_A_
        //          |_B
        //          |_C_
        //             |_D
        let root = graph.add_node(Node {
            base: Base::default(),
        });
        let d = graph.add_node(Node {
            base: Base::default(),
        });
        let c = graph.add_node(Node {
            base: Base {
                children: vec![d],
                ..Default::default()
            },
        });
        let b = graph.add_node(Node {
            base: Base::default(),
        });
        let a = graph.add_node(Node {
            base: Base {
                children: vec![b, c],
                ..Default::default()
            },
        });
        graph.link_nodes(a, root);

        dbg!(root, a, b, c, d);

        let link_scheme = graph.change_hierarchy_root(root, c);

        // C_
        //   |_D
        //   |_A_
        //       |_B
        //   |_Root
        assert_eq!(graph.root, c);

        assert_eq!(graph[graph.root].parent, Handle::NONE);
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
        assert_eq!(graph[graph.root].parent, Handle::NONE);
        assert_eq!(graph[graph.root].children, vec![a]);

        assert_eq!(graph[a].parent, root);
        assert_eq!(graph[a].children, vec![b, c]);

        assert_eq!(graph[b].parent, a);
        assert_eq!(graph[b].children, vec![]);

        assert_eq!(graph[c].parent, a);
        assert_eq!(graph[c].children, vec![d]);
    }
}
