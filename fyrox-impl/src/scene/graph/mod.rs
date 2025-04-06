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

//! Contains all methods and structures to create and manage scene graphs.
//!
//! Scene graph is the foundation of the engine. Graph is a hierarchical data
//! structure where each element called node. Each node can have zero to one parent
//! node, and any children nodes. Node with no parent node called root, with no
//! children nodes - leaf. Graphical representation can be something like this:
//!
//! ```text
//!     Root____
//!       |    |
//!       D    A___
//!       |    |  |
//!       E    C  B
//!     ............
//! ```
//!
//! This picture clearly shows relations between nodes. Such structure allows us
//! to create scenes of any complexity by just linking nodes with each other.
//! Connections between nodes are used to traverse tree, to calculate global
//! transforms, global visibility and many other things. Most interesting here -
//! is global transform calculation - it allows you to produce complex movements
//! just by linking nodes to each other. Good example of this is skeleton which
//! is used in skinning (animating 3d model by set of bones).

use crate::scene::node::NodeAsAny;
use crate::{
    asset::untyped::UntypedResource,
    core::{
        algebra::{Matrix4, Rotation3, UnitQuaternion, Vector2, Vector3},
        instant,
        log::{Log, MessageKind},
        math::{aabb::AxisAlignedBoundingBox, Matrix4Ext},
        pool::{ErasedHandle, Handle, MultiBorrowContext, Pool, Ticket},
        reflect::prelude::*,
        visitor::{Visit, VisitResult, Visitor},
    },
    graph::{AbstractSceneGraph, AbstractSceneNode, BaseSceneGraph, NodeHandleMap, SceneGraph},
    material::{MaterialResourceBinding, MaterialTextureBinding},
    resource::model::{Model, ModelResource, ModelResourceExtension},
    scene::{
        base::{NodeMessage, NodeMessageKind, NodeScriptMessage, SceneNodeId},
        camera::Camera,
        dim2::{self},
        graph::{
            event::{GraphEvent, GraphEventBroadcaster},
            physics::{PhysicsPerformanceStatistics, PhysicsWorld},
        },
        mesh::Mesh,
        navmesh,
        node::{container::NodeContainer, Node, NodeTrait, SyncContext, UpdateContext},
        pivot::Pivot,
        sound::context::SoundContext,
        transform::TransformBuilder,
    },
    script::ScriptTrait,
    utils::lightmap::{self, Lightmap},
};
use bitflags::bitflags;
use fxhash::{FxHashMap, FxHashSet};
use fyrox_core::pool::BorrowAs;
use fyrox_graph::SceneGraphNode;
use std::ops::{Deref, DerefMut};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Index, IndexMut},
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};

pub mod event;
pub mod physics;

/// Graph performance statistics. Allows you to find out "hot" parts of the scene graph, which
/// parts takes the most time to update.
#[derive(Clone, Default, Debug)]
pub struct GraphPerformanceStatistics {
    /// Amount of time that was needed to update global transform, visibility, and every other
    /// property of every object which depends on the state of a parent node.
    pub hierarchical_properties_time: Duration,

    /// Amount of time that was needed to synchronize state of the graph with the state of
    /// backing native objects (Rapier's rigid bodies, colliders, joints, sound sources, etc.)
    pub sync_time: Duration,

    /// Physics performance statistics.
    pub physics: PhysicsPerformanceStatistics,

    /// 2D Physics performance statistics.
    pub physics2d: PhysicsPerformanceStatistics,

    /// A time which was required to render sounds.
    pub sound_update_time: Duration,
}

impl GraphPerformanceStatistics {
    /// Returns total amount of time.
    pub fn total(&self) -> Duration {
        self.hierarchical_properties_time
            + self.sync_time
            + self.physics.total()
            + self.physics2d.total()
            + self.sound_update_time
    }
}

/// A helper type alias for node pool.
pub type NodePool = Pool<Node, NodeContainer>;

impl<T: NodeTrait> BorrowAs<Node, NodeContainer> for Handle<T> {
    type Target = T;

    fn borrow_as_ref(self, pool: &NodePool) -> Option<&T> {
        pool.try_borrow(self.transmute())
            .and_then(|n| NodeAsAny::as_any(n.0.deref()).downcast_ref::<T>())
    }

    fn borrow_as_mut(self, pool: &mut NodePool) -> Option<&mut T> {
        pool.try_borrow_mut(self.transmute())
            .and_then(|n| NodeAsAny::as_any_mut(n.0.deref_mut()).downcast_mut::<T>())
    }
}

/// See module docs.
#[derive(Debug, Reflect)]
pub struct Graph {
    #[reflect(hidden)]
    root: Handle<Node>,

    pool: NodePool,

    #[reflect(hidden)]
    stack: Vec<Handle<Node>>,

    /// Backing physics "world". It is responsible for the physics simulation.
    pub physics: PhysicsWorld,

    /// Backing 2D physics "world". It is responsible for the 2D physics simulation.
    pub physics2d: dim2::physics::PhysicsWorld,

    /// Backing sound context. It is responsible for sound rendering.
    #[reflect(hidden)]
    pub sound_context: SoundContext,

    /// Performance statistics of a last [`Graph::update`] call.
    #[reflect(hidden)]
    pub performance_statistics: GraphPerformanceStatistics,

    /// Allows you to "subscribe" for graph events.
    #[reflect(hidden)]
    pub event_broadcaster: GraphEventBroadcaster,

    /// Current lightmap.
    lightmap: Option<Lightmap>,

    #[reflect(hidden)]
    pub(crate) script_message_sender: Sender<NodeScriptMessage>,
    #[reflect(hidden)]
    pub(crate) script_message_receiver: Receiver<NodeScriptMessage>,

    #[reflect(hidden)]
    pub(crate) message_sender: Sender<NodeMessage>,
    #[reflect(hidden)]
    pub(crate) message_receiver: Receiver<NodeMessage>,

    instance_id_map: FxHashMap<SceneNodeId, Handle<Node>>,
}

impl Default for Graph {
    fn default() -> Self {
        let (script_message_sender, script_message_receiver) = channel();
        let (message_sender, message_receiver) = channel();

        Self {
            physics: PhysicsWorld::new(),
            physics2d: dim2::physics::PhysicsWorld::new(),
            root: Handle::NONE,
            pool: Pool::new(),
            stack: Vec::new(),
            sound_context: Default::default(),
            performance_statistics: Default::default(),
            event_broadcaster: Default::default(),
            script_message_receiver,
            message_sender,
            script_message_sender,
            lightmap: None,
            instance_id_map: Default::default(),
            message_receiver,
        }
    }
}

/// Sub-graph is a piece of graph that was extracted from a graph. It has ownership
/// over its nodes. It is used to temporarily take ownership of a sub-graph. This could
/// be used if you're making a scene editor with a command stack - once you reverted a command,
/// that created a complex nodes hierarchy (for example you loaded a model) you must store
/// all added nodes somewhere to be able to put nodes back into graph when user decide to re-do
/// command. Sub-graph allows you to do this without invalidating handles to nodes.
#[derive(Debug)]
pub struct SubGraph {
    /// A root node and its [ticket](/fyrox-core/model/struct.Ticket.html).
    pub root: (Ticket<Node>, Node),

    /// A set of descendant nodes with their tickets.
    pub descendants: Vec<(Ticket<Node>, Node)>,

    /// A handle to the parent node from which the sub-graph was extracted (it it parent node of
    /// the root of this sub-graph).
    pub parent: Handle<Node>,
}

fn remap_handles(old_new_mapping: &NodeHandleMap<Node>, dest_graph: &mut Graph) {
    // Iterate over instantiated nodes and remap handles.
    for (_, &new_node_handle) in old_new_mapping.inner().iter() {
        old_new_mapping.remap_handles(
            &mut dest_graph.pool[new_node_handle],
            &[TypeId::of::<UntypedResource>()],
        );
    }
}

/// Calculates local transform of a scene node without scaling.
pub fn isometric_local_transform(nodes: &NodePool, node: Handle<Node>) -> Matrix4<f32> {
    let transform = nodes[node].local_transform();
    TransformBuilder::new()
        .with_local_position(**transform.position())
        .with_local_rotation(**transform.rotation())
        .with_pre_rotation(**transform.pre_rotation())
        .with_post_rotation(**transform.post_rotation())
        .build()
        .matrix()
}

/// Calculates global transform of a scene node without scaling.
pub fn isometric_global_transform(nodes: &NodePool, node: Handle<Node>) -> Matrix4<f32> {
    let parent = nodes[node].parent();
    if parent.is_some() {
        isometric_global_transform(nodes, parent) * isometric_local_transform(nodes, node)
    } else {
        isometric_local_transform(nodes, node)
    }
}

// Clears all information about parent-child relations of a given node. This is needed in some
// cases (mostly when copying a node), because `Graph::add_node` uses children list to attach
// children to the given node, and when copying a node it is important that this step is skipped.
fn clear_links(mut node: Node) -> Node {
    node.children.clear();
    node.parent = Handle::NONE;
    node
}

/// A set of switches that allows you to disable a particular step of graph update pipeline.
#[derive(Clone, PartialEq, Eq)]
pub struct GraphUpdateSwitches {
    /// Enables or disables update of the 2D physics.
    pub physics2d: bool,
    /// Enables or disables update of the 3D physics.
    pub physics: bool,
    /// A set of nodes that will be updated, everything else won't be updated.
    pub node_overrides: Option<FxHashSet<Handle<Node>>>,
    /// Enables or disables deletion of the nodes with ended lifetime (lifetime <= 0.0). If set to `false` the lifetime
    /// of the nodes won't be changed.
    pub delete_dead_nodes: bool,
    /// Whether the graph update is paused or not. Paused graphs won't be updated and their sound content will be also paused
    /// so it won't emit any sounds.
    pub paused: bool,
}

impl Default for GraphUpdateSwitches {
    fn default() -> Self {
        Self {
            physics2d: true,
            physics: true,
            node_overrides: Default::default(),
            delete_dead_nodes: true,
            paused: false,
        }
    }
}

impl Graph {
    /// Creates new graph instance with single root node.
    #[inline]
    pub fn new() -> Self {
        let (script_message_sender, script_message_receiver) = channel();
        let (message_sender, message_receiver) = channel();

        // Create root node.
        let mut root_node = Pivot::default();
        let instance_id = root_node.instance_id;
        root_node.set_name("__ROOT__");

        // Add it to the pool.
        let mut pool = Pool::new();
        let root = pool.spawn(Node::new(root_node));
        pool[root].on_connected_to_graph(
            root,
            message_sender.clone(),
            script_message_sender.clone(),
        );

        let instance_id_map = FxHashMap::from_iter([(instance_id, root)]);

        Self {
            physics: Default::default(),
            stack: Vec::new(),
            root,
            pool,
            physics2d: Default::default(),
            sound_context: SoundContext::new(),
            performance_statistics: Default::default(),
            event_broadcaster: Default::default(),
            script_message_receiver,
            message_sender,
            script_message_sender,
            lightmap: None,
            instance_id_map,
            message_receiver,
        }
    }

    /// Creates a new graph using a hierarchy of nodes specified by the `root`.
    pub fn from_hierarchy(root: Handle<Node>, other_graph: &Self) -> Self {
        let mut graph = Self::default();
        other_graph.copy_node(
            root,
            &mut graph,
            &mut |_, _| true,
            &mut |_, _| {},
            &mut |_, _, _| {},
        );
        graph
    }

    /// Sets new root of the graph and attaches the old root to the new root. Old root becomes a child
    /// node of the new root.
    pub fn change_root_node(&mut self, root: Node) {
        let prev_root = self.root;
        self.root = Handle::NONE;
        let handle = self.add_node(root);
        assert_eq!(self.root, handle);
        self.link_nodes(prev_root, handle);
    }

    /// Tries to find references of the given node in other scene nodes. It could be used to check if the node is
    /// used by some other scene node or not. Returns an array of nodes, that references the given node. This method
    /// is reflection-based, so it is quite slow and should not be used every frame.
    pub fn find_references_to(&self, target: Handle<Node>) -> Vec<Handle<Node>> {
        let mut references = Vec::new();
        for (node_handle, node) in self.pair_iter() {
            (node as &dyn Reflect).apply_recursively(
                &mut |object| {
                    object.as_any(&mut |any| {
                        if let Some(handle) = any.downcast_ref::<Handle<Node>>() {
                            if *handle == target {
                                references.push(node_handle);
                            }
                        }
                    })
                },
                &[],
            );
        }
        references
    }

    /// Sets global position of a scene node. Internally, this method converts the given position
    /// to the local space of the parent node of the given scene node and sets it as local position
    /// of the node. In other words, this method does not modify global position itself, but calculates
    /// new local position of the given node so that its global position will be as requested.
    ///
    /// ## Important
    ///
    /// This method relies on pre-calculated global transformation of the hierarchy. This may give
    /// unexpected results if you've modified transforms of ancestors in the hierarchy of the node
    /// and then called this method. This happens because global transform calculation is deferred
    /// to the end of the frame. If you want to ensure that everything works as expected, call
    /// [`Self::update_hierarchical_data`] before calling this method. It is not called automatically,
    /// because it is quite heavy, and in most cases this method works ok without it.
    pub fn set_global_position(&mut self, node_handle: Handle<Node>, position: Vector3<f32>) {
        let (node, parent) = self
            .pool
            .try_borrow_dependant_mut(node_handle, |node| node.parent());
        if let Some(node) = node {
            if let Some(parent) = parent {
                let relative_position = parent
                    .global_transform()
                    .try_inverse()
                    .unwrap_or_default()
                    .transform_point(&position.into())
                    .coords;
                node.local_transform_mut().set_position(relative_position);
            } else {
                node.local_transform_mut().set_position(position);
            }
            self.update_hierarchical_data_for_descendants(node_handle);
        }
    }

    /// Sets global rotation of a scene node. Internally, this method converts the given rotation
    /// to the local space of the parent node of the given scene node and sets it as local rotation
    /// of the node. In other words, this method does not modify global rotation itself, but calculates
    /// new local rotation of the given node so that its global rotation will be as requested.
    ///
    /// ## Important
    ///
    /// This method relies on pre-calculated global transformation of the hierarchy. This may give
    /// unexpected results if you've modified transforms of ancestors in the hierarchy of the node
    /// and then called this method. This happens because global transform calculation is deferred
    /// to the end of the frame. If you want to ensure that everything works as expected, call
    /// [`Self::update_hierarchical_data`] before calling this method. It is not called automatically,
    /// because it is quite heavy, and in most cases this method works ok without it.
    pub fn set_global_rotation(&mut self, node: Handle<Node>, rotation: UnitQuaternion<f32>) {
        let (node, parent) = self
            .pool
            .try_borrow_dependant_mut(node, |node| node.parent());
        if let Some(node) = node {
            if let Some(parent) = parent {
                let basis = parent
                    .global_transform()
                    .try_inverse()
                    .unwrap_or_default()
                    .basis();
                let relative_rotation = UnitQuaternion::from_matrix(&basis) * rotation;
                node.local_transform_mut().set_rotation(relative_rotation);
            } else {
                node.local_transform_mut().set_rotation(rotation);
            }
        }
    }

    /// Tries to borrow mutable references to two nodes at the same time by given handles. Will
    /// panic if handles overlaps (points to same node).
    #[inline]
    pub fn get_two_mut(&mut self, nodes: (Handle<Node>, Handle<Node>)) -> (&mut Node, &mut Node) {
        self.pool.borrow_two_mut(nodes)
    }

    /// Tries to borrow mutable references to three nodes at the same time by given handles. Will
    /// return Err of handles overlaps (points to same node).
    #[inline]
    pub fn get_three_mut(
        &mut self,
        nodes: (Handle<Node>, Handle<Node>, Handle<Node>),
    ) -> (&mut Node, &mut Node, &mut Node) {
        self.pool.borrow_three_mut(nodes)
    }

    /// Tries to borrow mutable references to four nodes at the same time by given handles. Will
    /// panic if handles overlaps (points to same node).
    #[inline]
    pub fn get_four_mut(
        &mut self,
        nodes: (Handle<Node>, Handle<Node>, Handle<Node>, Handle<Node>),
    ) -> (&mut Node, &mut Node, &mut Node, &mut Node) {
        self.pool.borrow_four_mut(nodes)
    }

    /// Returns root node of current graph.
    #[inline]
    pub fn get_root(&self) -> Handle<Node> {
        self.root
    }

    /// Tries to mutably borrow a node, returns Some(node) if the handle is valid, None - otherwise.
    #[inline]
    pub fn try_get_mut(&mut self, handle: Handle<Node>) -> Option<&mut Node> {
        self.pool.try_borrow_mut(handle)
    }

    /// Begins multi-borrow that allows you borrow to as many shared references to the graph
    /// nodes as you need and only one mutable reference to a node. See
    /// [`MultiBorrowContext::try_get`] for more info.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use fyrox_impl::{
    /// #     core::pool::Handle,
    /// #     scene::{base::BaseBuilder, graph::Graph, node::Node, pivot::PivotBuilder},
    /// # };
    /// #
    /// let mut graph = Graph::new();
    ///
    /// let handle1 = PivotBuilder::new(BaseBuilder::new()).build(&mut graph);
    /// let handle2 = PivotBuilder::new(BaseBuilder::new()).build(&mut graph);
    /// let handle3 = PivotBuilder::new(BaseBuilder::new()).build(&mut graph);
    /// let handle4 = PivotBuilder::new(BaseBuilder::new()).build(&mut graph);
    ///
    /// let mut ctx = graph.begin_multi_borrow();
    ///
    /// let node1 = ctx.try_get(handle1);
    /// let node2 = ctx.try_get(handle2);
    /// let node3 = ctx.try_get(handle3);
    /// let node4 = ctx.try_get(handle4);
    ///
    /// assert!(node1.is_ok());
    /// assert!(node2.is_ok());
    /// assert!(node3.is_ok());
    /// assert!(node4.is_ok());
    ///
    /// // An attempt to borrow the same node twice as immutable and mutable will fail.
    /// assert!(ctx.try_get_mut(handle1).is_err());
    /// ```
    #[inline]
    pub fn begin_multi_borrow(&mut self) -> MultiBorrowContext<Node, NodeContainer> {
        self.pool.begin_multi_borrow()
    }

    /// Links specified child with specified parent while keeping the
    /// child's global position and rotation.
    #[inline]
    pub fn link_nodes_keep_global_transform(&mut self, child: Handle<Node>, parent: Handle<Node>) {
        let parent_global_transform_inv = self.pool[parent]
            .global_transform()
            .try_inverse()
            .unwrap_or_default();
        let child_global_transform = self.pool[child].global_transform();
        let relative_transform = parent_global_transform_inv * child_global_transform;
        let local_position = relative_transform.position();
        let parent_inv_global_rotation = self.global_rotation(parent).inverse();
        let local_rotation = parent_inv_global_rotation * self.global_rotation(child);
        let local_scale = self
            .global_scale(child)
            .component_div(&self.global_scale(parent));
        self.pool[child]
            .local_transform_mut()
            .set_position(local_position)
            .set_rotation(local_rotation)
            .set_scale(local_scale);
        self.link_nodes(child, parent);
    }

    /// Searches for a **first** node with a script of the given type `S` in the hierarchy starting from the
    /// given `root_node`.
    #[inline]
    pub fn find_first_by_script<S>(&self, root_node: Handle<Node>) -> Option<(Handle<Node>, &Node)>
    where
        S: ScriptTrait,
    {
        self.find(root_node, &mut |node| {
            for script in &node.scripts {
                if script.as_ref().and_then(|s| s.cast::<S>()).is_some() {
                    return true;
                }
            }
            false
        })
    }

    /// Creates deep copy of node with all children. This is relatively heavy operation!
    /// In case if any error happened it returns `Handle::NONE`. This method can be used
    /// to create exact copy of given node hierarchy. For example you can prepare rocket
    /// model: case of rocket will be mesh, and fire from nozzle will be particle system,
    /// and when you fire from rocket launcher you just need to create a copy of such
    /// "prefab".
    ///
    /// # Implementation notes
    ///
    /// Returns tuple where first element is handle to copy of node, and second element -
    /// old-to-new hash map, which can be used to easily find copy of node by its original.
    ///
    /// Filter allows to exclude some nodes from copied hierarchy. It must return false for
    /// odd nodes. Filtering applied only to descendant nodes.
    #[inline]
    pub fn copy_node<F, Pre, Post>(
        &self,
        node_handle: Handle<Node>,
        dest_graph: &mut Graph,
        filter: &mut F,
        pre_process_callback: &mut Pre,
        post_process_callback: &mut Post,
    ) -> (Handle<Node>, NodeHandleMap<Node>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
        Pre: FnMut(Handle<Node>, &mut Node),
        Post: FnMut(Handle<Node>, Handle<Node>, &mut Node),
    {
        let mut old_new_mapping = NodeHandleMap::default();
        let root_handle = self.copy_node_raw(
            node_handle,
            dest_graph,
            &mut old_new_mapping,
            filter,
            pre_process_callback,
            post_process_callback,
        );

        remap_handles(&old_new_mapping, dest_graph);

        (root_handle, old_new_mapping)
    }

    /// Creates deep copy of node with all children. This is relatively heavy operation!
    /// In case if any error happened it returns `Handle::NONE`. This method can be used
    /// to create exact copy of given node hierarchy. For example you can prepare rocket
    /// model: case of rocket will be mesh, and fire from nozzle will be particle system,
    /// and when you fire from rocket launcher you just need to create a copy of such
    /// "prefab".
    ///
    /// # Implementation notes
    ///
    /// Returns tuple where first element is handle to copy of node, and second element -
    /// old-to-new hash map, which can be used to easily find copy of node by its original.
    ///
    /// Filter allows to exclude some nodes from copied hierarchy. It must return false for
    /// odd nodes. Filtering applied only to descendant nodes.
    #[inline]
    pub fn copy_node_inplace<F>(
        &mut self,
        node_handle: Handle<Node>,
        filter: &mut F,
    ) -> (Handle<Node>, NodeHandleMap<Node>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let mut old_new_mapping = NodeHandleMap::default();

        let to_copy = self
            .traverse_iter(node_handle)
            .map(|(descendant_handle, descendant)| (descendant_handle, descendant.children.clone()))
            .collect::<Vec<_>>();

        let mut root_handle = Handle::NONE;

        for (parent, children) in to_copy.iter() {
            // Copy parent first.
            let parent_copy = clear_links(self.pool[*parent].clone_box());
            let parent_copy_handle = self.add_node(parent_copy);
            old_new_mapping.insert(*parent, parent_copy_handle);

            if root_handle.is_none() {
                root_handle = parent_copy_handle;
            }

            // Copy children and link to new parent.
            for &child in children {
                if filter(child, &self.pool[child]) {
                    let child_copy = clear_links(self.pool[child].clone_box());
                    let child_copy_handle = self.add_node(child_copy);
                    old_new_mapping.insert(child, child_copy_handle);
                    self.link_nodes(child_copy_handle, parent_copy_handle);
                }
            }
        }

        remap_handles(&old_new_mapping, self);

        (root_handle, old_new_mapping)
    }

    /// Creates copy of a node and breaks all connections with other nodes. Keep in mind that
    /// this method may give unexpected results when the node has connections with other nodes.
    /// For example if you'll try to copy a skinned mesh, its copy won't be skinned anymore -
    /// you'll get just a "shallow" mesh. Also unlike [copy_node](struct.Graph.html#method.copy_node)
    /// this method returns copied node directly, it does not inserts it in any graph.
    #[inline]
    pub fn copy_single_node(&self, node_handle: Handle<Node>) -> Node {
        let node = &self.pool[node_handle];
        let mut clone = clear_links(node.clone_box());
        if let Some(ref mut mesh) = clone.cast_mut::<Mesh>() {
            for surface in mesh.surfaces_mut() {
                surface.bones.clear();
            }
        }
        clone
    }

    fn copy_node_raw<F, Pre, Post>(
        &self,
        root_handle: Handle<Node>,
        dest_graph: &mut Graph,
        old_new_mapping: &mut NodeHandleMap<Node>,
        filter: &mut F,
        pre_process_callback: &mut Pre,
        post_process_callback: &mut Post,
    ) -> Handle<Node>
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
        Pre: FnMut(Handle<Node>, &mut Node),
        Post: FnMut(Handle<Node>, Handle<Node>, &mut Node),
    {
        let src_node = &self.pool[root_handle];
        let mut dest_node = clear_links(src_node.clone_box());
        pre_process_callback(root_handle, &mut dest_node);
        let dest_copy_handle = dest_graph.add_node(dest_node);
        old_new_mapping.insert(root_handle, dest_copy_handle);
        for &src_child_handle in src_node.children() {
            if filter(src_child_handle, &self.pool[src_child_handle]) {
                let dest_child_handle = self.copy_node_raw(
                    src_child_handle,
                    dest_graph,
                    old_new_mapping,
                    filter,
                    pre_process_callback,
                    post_process_callback,
                );
                if !dest_child_handle.is_none() {
                    dest_graph.link_nodes(dest_child_handle, dest_copy_handle);
                }
            }
        }
        post_process_callback(
            dest_copy_handle,
            root_handle,
            &mut dest_graph[dest_copy_handle],
        );
        dest_copy_handle
    }

    fn restore_dynamic_node_data(&mut self) {
        for (handle, node) in self.pool.pair_iter_mut() {
            node.on_connected_to_graph(
                handle,
                self.message_sender.clone(),
                self.script_message_sender.clone(),
            );
        }
    }

    // Fix property flags for scenes made before inheritance system was fixed. By default, all inheritable properties
    // must be marked as modified in nodes without any parent resource.
    pub(crate) fn mark_ancestor_nodes_as_modified(&mut self) {
        for node in self.linear_iter_mut() {
            if node.resource.is_none() {
                node.mark_inheritable_variables_as_modified();
            }
        }
    }

    pub(crate) fn resolve(&mut self) {
        Log::writeln(MessageKind::Information, "Resolving graph...");

        self.restore_dynamic_node_data();
        self.mark_ancestor_nodes_as_modified();
        self.restore_original_handles_and_inherit_properties(
            &[TypeId::of::<navmesh::Container>()],
            |resource_node, node| {
                node.inv_bind_pose_transform = resource_node.inv_bind_pose_transform;
            },
        );
        self.update_hierarchical_data();
        let instances = self.restore_integrity(|model, model_data, handle, dest_graph| {
            ModelResource::instantiate_from(model, model_data, handle, dest_graph, &mut |_, _| {})
        });
        self.remap_handles(&instances);

        // Update cube maps for sky boxes.
        for node in self.linear_iter_mut() {
            if let Some(camera) = node.cast_mut::<Camera>() {
                if let Some(skybox) = camera.skybox_mut() {
                    Log::verify(skybox.create_cubemap());
                }
            }
        }

        self.apply_lightmap();

        Log::writeln(MessageKind::Information, "Graph resolved successfully!");
    }

    /// Tries to set new lightmap to scene.
    pub fn set_lightmap(&mut self, lightmap: Lightmap) -> Result<Option<Lightmap>, &'static str> {
        // Assign textures to surfaces.
        for (handle, lightmaps) in lightmap.map.iter() {
            if let Some(mesh) = self[*handle].cast_mut::<Mesh>() {
                if mesh.surfaces().len() != lightmaps.len() {
                    return Err("failed to set lightmap, surface count mismatch");
                }

                for (surface, entry) in mesh.surfaces_mut().iter_mut().zip(lightmaps) {
                    // This unwrap() call must never panic in normal conditions, because texture wrapped in Option
                    // only to implement Default trait to be serializable.
                    let texture = entry.texture.clone().unwrap();
                    let mut material_state = surface.material().state();
                    if let Some(material) = material_state.data() {
                        material.bind(
                            "lightmapTexture",
                            MaterialResourceBinding::Texture(MaterialTextureBinding {
                                value: Some(texture),
                            }),
                        );
                    }
                }
            }
        }
        Ok(std::mem::replace(&mut self.lightmap, Some(lightmap)))
    }

    /// Returns current lightmap.
    pub fn lightmap(&self) -> Option<&Lightmap> {
        self.lightmap.as_ref()
    }

    fn apply_lightmap(&mut self) {
        // Re-apply lightmap if any. This has to be done after resolve because we must patch surface
        // data at this stage, but if we'd do this before we wouldn't be able to do this because
        // meshes contains invalid surface data.
        if let Some(lightmap) = self.lightmap.as_mut() {
            // Patch surface data first. To do this we gather all surface data instances and
            // look in patch data if we have patch for data.
            let mut unique_data_set = FxHashMap::default();
            for &handle in lightmap.map.keys() {
                if let Some(mesh) = self.pool[handle].cast_mut::<Mesh>() {
                    for surface in mesh.surfaces() {
                        let data = surface.data();
                        unique_data_set.entry(data.key()).or_insert(data);
                    }
                }
            }

            for (_, data) in unique_data_set.into_iter() {
                let mut data = data.data_ref();

                if let Some(patch) = lightmap.patches.get(&data.content_hash()) {
                    lightmap::apply_surface_data_patch(&mut data, &patch.0);
                } else {
                    Log::writeln(
                        MessageKind::Warning,
                        "Failed to get surface data patch while resolving lightmap!\
                    This means that surface has changed and lightmap must be regenerated!",
                    );
                }
            }

            // Apply textures.
            for (&handle, entries) in lightmap.map.iter_mut() {
                if let Some(mesh) = self.pool[handle].cast_mut::<Mesh>() {
                    for (entry, surface) in entries.iter_mut().zip(mesh.surfaces_mut()) {
                        let mut material_state = surface.material().state();
                        if let Some(material) = material_state.data() {
                            material.bind(
                                "lightmapTexture",
                                MaterialResourceBinding::Texture(MaterialTextureBinding {
                                    value: entry.texture.clone(),
                                }),
                            );
                        }
                    }
                }
            }
        }
    }

    /// Tries to compute combined axis-aligned bounding box (AABB) in world-space of the hierarchy starting from the given
    /// scene node. It will return [`None`] if the scene node handle is invalid, otherwise it will return AABB that enclosing
    /// all the nodes in the hierarchy.
    pub fn aabb_of_descendants<F>(
        &self,
        root: Handle<Node>,
        mut filter: F,
    ) -> Option<AxisAlignedBoundingBox>
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        fn aabb_of_descendants_recursive<F>(
            graph: &Graph,
            node: Handle<Node>,
            filter: &mut F,
        ) -> Option<AxisAlignedBoundingBox>
        where
            F: FnMut(Handle<Node>, &Node) -> bool,
        {
            graph.try_get(node).and_then(|n| {
                if filter(node, n) {
                    let mut aabb = n.local_bounding_box();
                    if aabb.is_invalid_or_degenerate() {
                        aabb = AxisAlignedBoundingBox::collapsed().transform(&n.global_transform());
                    } else {
                        aabb = aabb.transform(&n.global_transform());
                    }
                    for child in n.children() {
                        if let Some(child_aabb) =
                            aabb_of_descendants_recursive(graph, *child, filter)
                        {
                            aabb.add_box(child_aabb);
                        }
                    }
                    Some(aabb)
                } else {
                    None
                }
            })
        }
        aabb_of_descendants_recursive(self, root, &mut filter)
    }

    pub(crate) fn update_enabled_flag_recursively(nodes: &NodePool, node_handle: Handle<Node>) {
        let Some(node) = nodes.try_borrow(node_handle) else {
            return;
        };

        let parent_enabled = nodes
            .try_borrow(node.parent())
            .is_none_or(|p| p.is_globally_enabled());
        node.global_enabled.set(parent_enabled && node.is_enabled());

        for &child in node.children() {
            Self::update_enabled_flag_recursively(nodes, child);
        }
    }

    pub(crate) fn update_visibility_recursively(nodes: &NodePool, node_handle: Handle<Node>) {
        let Some(node) = nodes.try_borrow(node_handle) else {
            return;
        };

        let parent_visibility = nodes
            .try_borrow(node.parent())
            .is_none_or(|p| p.global_visibility());
        node.global_visibility
            .set(parent_visibility && node.visibility());

        for &child in node.children() {
            Self::update_visibility_recursively(nodes, child);
        }
    }

    pub(crate) fn update_global_transform_recursively(
        nodes: &NodePool,
        sound_context: &mut SoundContext,
        physics: &mut PhysicsWorld,
        physics2d: &mut dim2::physics::PhysicsWorld,
        node_handle: Handle<Node>,
    ) {
        let Some(node) = nodes.try_borrow(node_handle) else {
            return;
        };

        let parent_global_transform = if let Some(parent) = nodes.try_borrow(node.parent()) {
            parent.global_transform()
        } else {
            Matrix4::identity()
        };

        let new_global_transform = parent_global_transform * node.local_transform().matrix();

        // TODO: Detect changes from user code here.
        node.on_global_transform_changed(
            &new_global_transform,
            &mut SyncContext {
                nodes,
                physics,
                physics2d,
                sound_context,
                switches: None,
            },
        );

        node.global_transform.set(new_global_transform);

        for &child in node.children() {
            Self::update_global_transform_recursively(
                nodes,
                sound_context,
                physics,
                physics2d,
                child,
            );
        }
    }

    /// Calculates local and global transform, global visibility for each node in graph starting from the
    /// specified node and down the tree. The main use case of the method is to update global position (etc.)
    /// of an hierarchy of the nodes of some new prefab instance.
    ///
    /// # Important Notes
    ///
    /// This method could be slow for large hierarchies. You should call it only when absolutely needed.
    #[inline]
    pub fn update_hierarchical_data_for_descendants(&mut self, node_handle: Handle<Node>) {
        Self::update_hierarchical_data_recursively(
            &self.pool,
            &mut self.sound_context,
            &mut self.physics,
            &mut self.physics2d,
            node_handle,
        );
    }

    /// Calculates local and global transform, global visibility for each node in graph starting from the
    /// specified node and down the tree. The main use case of the method is to update global position (etc.)
    /// of an hierarchy of the nodes of some new prefab instance.
    ///
    /// # Important Notes
    ///
    /// This method could be slow for large graph. You should call it only when absolutely needed.
    #[inline]
    pub fn update_hierarchical_data(&mut self) {
        self.update_hierarchical_data_for_descendants(self.root);
    }

    pub(crate) fn update_hierarchical_data_recursively(
        nodes: &NodePool,
        sound_context: &mut SoundContext,
        physics: &mut PhysicsWorld,
        physics2d: &mut dim2::physics::PhysicsWorld,
        node_handle: Handle<Node>,
    ) {
        Self::update_global_transform_recursively(
            nodes,
            sound_context,
            physics,
            physics2d,
            node_handle,
        );
        Self::update_enabled_flag_recursively(nodes, node_handle);
        Self::update_visibility_recursively(nodes, node_handle);
    }

    // This method processes messages from scene nodes and propagates changes on descendant nodes
    // in the hierarchy. Scene nodes have global transform, visibility and enabled flags and their
    // values depend on the values of ancestors in the hierarchy. This method uses optimized changes
    // propagation that propagates changes on small "chains" of nodes instead of updating the entire
    // graph. This is much faster since most scene nodes remain unchanged most of the time.
    //
    // Performance of this method is detached from the amount of scene nodes in the graph and only
    // correlates with the amount of changing nodes, allowing to have large scene graphs with tons
    // of static nodes.
    pub(crate) fn process_node_messages(&mut self, switches: Option<&GraphUpdateSwitches>) {
        bitflags! {
            #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
            struct Flags: u8 {
                const NONE = 0;
                const TRANSFORM_CHANGED = 0b0001;
                const VISIBILITY_CHANGED = 0b0010;
                const ENABLED_FLAG_CHANGED = 0b0100;
            }
        }

        let mut visited_flags = vec![Flags::NONE; self.pool.get_capacity() as usize];
        let mut roots = FxHashMap::default();

        while let Ok(message) = self.message_receiver.try_recv() {
            if let NodeMessageKind::TransformChanged = message.kind {
                if let Some(node) = self.pool.try_borrow(message.node) {
                    node.on_local_transform_changed(&mut SyncContext {
                        nodes: &self.pool,
                        physics: &mut self.physics,
                        physics2d: &mut self.physics2d,
                        sound_context: &mut self.sound_context,
                        switches,
                    })
                }
            }

            // Prepare for hierarchy propagation.
            let message_flag = match message.kind {
                NodeMessageKind::TransformChanged => Flags::TRANSFORM_CHANGED,
                NodeMessageKind::VisibilityChanged => Flags::VISIBILITY_CHANGED,
                NodeMessageKind::EnabledFlagChanged => Flags::ENABLED_FLAG_CHANGED,
            };

            let visit_flags = &mut visited_flags[message.node.index() as usize];

            if visit_flags.contains(message_flag) {
                continue;
            }

            visit_flags.insert(message_flag);

            roots
                .entry(message.node)
                .or_insert(Flags::NONE)
                .insert(message_flag);

            // Mark the entire hierarchy as visited.
            fn traverse_recursive(
                graph: &Graph,
                from: Handle<Node>,
                func: &mut impl FnMut(Handle<Node>),
            ) {
                func(from);
                if let Some(node) = graph.try_get(from) {
                    for &child in node.children() {
                        traverse_recursive(graph, child, func)
                    }
                }
            }

            traverse_recursive(self, message.node, &mut |h| {
                visited_flags[h.index() as usize].insert(message_flag);

                // Remove a descendant from the list of potential roots.
                if h != message.node {
                    if let Some(flags) = roots.get_mut(&h) {
                        flags.remove(message_flag);

                        if flags.is_empty() {
                            roots.remove(&h);
                        }
                    }
                }
            })
        }

        for (node, flags) in roots {
            if flags.contains(Flags::TRANSFORM_CHANGED) {
                Self::update_global_transform_recursively(
                    &self.pool,
                    &mut self.sound_context,
                    &mut self.physics,
                    &mut self.physics2d,
                    node,
                );
            }
            // All these calls could be combined into one with the above, but visibility/enabled
            // flags changes so rare, that it isn't worth spending CPU cycles on useless checks.
            if flags.contains(Flags::VISIBILITY_CHANGED) {
                Self::update_visibility_recursively(&self.pool, node);
            }
            if flags.contains(Flags::ENABLED_FLAG_CHANGED) {
                Self::update_enabled_flag_recursively(&self.pool, node)
            }
        }
    }

    fn sync_native(&mut self, switches: &GraphUpdateSwitches) {
        let mut sync_context = SyncContext {
            nodes: &self.pool,
            physics: &mut self.physics,
            physics2d: &mut self.physics2d,
            sound_context: &mut self.sound_context,
            switches: Some(switches),
        };

        for (handle, node) in self.pool.pair_iter() {
            node.sync_native(handle, &mut sync_context);
        }
    }

    fn update_node(
        &mut self,
        handle: Handle<Node>,
        frame_size: Vector2<f32>,
        dt: f32,
        delete_dead_nodes: bool,
    ) {
        if let Some((ticket, mut node)) = self.pool.try_take_reserve(handle) {
            let mut is_alive = node.is_alive();

            if node.is_globally_enabled() {
                node.update(&mut UpdateContext {
                    frame_size,
                    dt,
                    nodes: &mut self.pool,
                    physics: &mut self.physics,
                    physics2d: &mut self.physics2d,
                    sound_context: &mut self.sound_context,
                });

                if delete_dead_nodes {
                    if let Some(lifetime) = node.lifetime.get_value_mut_silent().as_mut() {
                        *lifetime -= dt;
                        if *lifetime <= 0.0 {
                            is_alive = false;
                        }
                    }
                }
            }

            self.pool.put_back(ticket, node);

            if !is_alive && delete_dead_nodes {
                self.remove_node(handle);
            }
        }
    }

    /// Updates nodes in the graph using given delta time.
    ///
    /// # Update Switches
    ///
    /// Update switches allows you to disable update for parts of the update pipeline, it could be useful for editors
    /// where you need to have preview mode to update only specific set of nodes, etc.
    pub fn update(&mut self, frame_size: Vector2<f32>, dt: f32, switches: GraphUpdateSwitches) {
        self.sound_context.state().pause(switches.paused);

        if switches.paused {
            return;
        }

        let last_time = instant::Instant::now();
        self.process_node_messages(Some(&switches));
        self.performance_statistics.hierarchical_properties_time =
            instant::Instant::now() - last_time;

        let last_time = instant::Instant::now();
        self.sync_native(&switches);
        self.performance_statistics.sync_time = instant::Instant::now() - last_time;

        if switches.physics {
            self.physics.performance_statistics.reset();
            self.physics.update(dt);
            self.performance_statistics.physics = self.physics.performance_statistics.clone();
        }

        if switches.physics2d {
            self.physics2d.performance_statistics.reset();
            self.physics2d.update(dt);
            self.performance_statistics.physics2d = self.physics2d.performance_statistics.clone();
        }

        self.performance_statistics.sound_update_time =
            self.sound_context.state().full_render_duration();

        if let Some(overrides) = switches.node_overrides.as_ref() {
            for handle in overrides {
                self.update_node(*handle, frame_size, dt, switches.delete_dead_nodes);
            }
        } else {
            for i in 0..self.pool.get_capacity() {
                self.update_node(
                    self.pool.handle_from_index(i),
                    frame_size,
                    dt,
                    switches.delete_dead_nodes,
                );
            }
        }
    }

    /// Returns capacity of internal pool. Can be used to iterate over all **potentially**
    /// available indices and try to convert them to handles.
    ///
    /// ```
    /// # use fyrox_impl::scene::node::Node;
    /// # use fyrox_impl::scene::graph::Graph;
    /// # use fyrox_impl::scene::pivot::Pivot;
    /// # use fyrox_graph::BaseSceneGraph;
    /// let mut graph = Graph::new();
    /// graph.add_node(Node::new(Pivot::default()));
    /// graph.add_node(Node::new(Pivot::default()));
    /// for i in 0..graph.capacity() {
    ///     let handle = graph.handle_from_index(i);
    ///     if handle.is_some() {
    ///         let node = &mut graph[handle];
    ///         // Do something with node.
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.pool.get_capacity()
    }

    /// Makes new handle from given index. Handle will be none if index was either out-of-bounds
    /// or point to a vacant pool entry.
    ///
    /// ```
    /// # use fyrox_impl::scene::node::Node;
    /// # use fyrox_impl::scene::graph::Graph;
    /// # use fyrox_impl::scene::pivot::Pivot;
    /// # use fyrox_graph::BaseSceneGraph;
    /// let mut graph = Graph::new();
    /// graph.add_node(Node::new(Pivot::default()));
    /// graph.add_node(Node::new(Pivot::default()));
    /// for i in 0..graph.capacity() {
    ///     let handle = graph.handle_from_index(i);
    ///     if handle.is_some() {
    ///         let node = &mut graph[handle];
    ///         // Do something with node.
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn handle_from_index(&self, index: u32) -> Handle<Node> {
        self.pool.handle_from_index(index)
    }

    /// Generates a set of handles that could be used to spawn a set of nodes.
    #[inline]
    pub fn generate_free_handles(&self, amount: usize) -> Vec<Handle<Node>> {
        self.pool.generate_free_handles(amount)
    }

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    #[inline]
    pub fn linear_iter(&self) -> impl Iterator<Item = &Node> {
        self.pool.iter()
    }

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    #[inline]
    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (Handle<Node>, &mut Node)> {
        self.pool.pair_iter_mut()
    }

    /// Extracts node from graph and reserves its handle. It is used to temporarily take
    /// ownership over node, and then put node back using given ticket. Extracted node is
    /// detached from its parent!
    #[inline]
    pub fn take_reserve(&mut self, handle: Handle<Node>) -> (Ticket<Node>, Node) {
        self.isolate_node(handle);
        let (ticket, node) = self.take_reserve_internal(handle);
        self.instance_id_map.remove(&node.instance_id);
        (ticket, node)
    }

    pub(crate) fn take_reserve_internal(&mut self, handle: Handle<Node>) -> (Ticket<Node>, Node) {
        let (ticket, mut node) = self.pool.take_reserve(handle);
        self.instance_id_map.remove(&node.instance_id);
        node.on_removed_from_graph(self);
        (ticket, node)
    }

    /// Puts node back by given ticket. Attaches back to root node of graph.
    #[inline]
    pub fn put_back(&mut self, ticket: Ticket<Node>, node: Node) -> Handle<Node> {
        let handle = self.put_back_internal(ticket, node);
        self.link_nodes(handle, self.root);
        handle
    }

    pub(crate) fn put_back_internal(&mut self, ticket: Ticket<Node>, node: Node) -> Handle<Node> {
        let instance_id = node.instance_id;
        let handle = self.pool.put_back(ticket, node);
        self.instance_id_map.insert(instance_id, handle);
        handle
    }

    /// Makes node handle vacant again.
    #[inline]
    pub fn forget_ticket(&mut self, ticket: Ticket<Node>, node: Node) -> Node {
        self.pool.forget_ticket(ticket);
        node
    }

    /// Extracts sub-graph starting from a given node. All handles to extracted nodes
    /// becomes reserved and will be marked as "occupied", an attempt to borrow a node
    /// at such handle will result in panic!. Please note that root node will be
    /// detached from its parent!
    #[inline]
    pub fn take_reserve_sub_graph(&mut self, root: Handle<Node>) -> SubGraph {
        // Take out descendants first.
        let mut descendants = Vec::new();
        let root_ref = &mut self[root];
        let mut stack = root_ref.children().to_vec();
        let parent = root_ref.parent;
        while let Some(handle) = stack.pop() {
            stack.extend_from_slice(self[handle].children());
            descendants.push(self.take_reserve_internal(handle));
        }

        SubGraph {
            // Root must be extracted with detachment from its parent (if any).
            root: self.take_reserve(root),
            descendants,
            parent,
        }
    }

    /// Puts previously extracted sub-graph into graph. Handles to nodes will become valid
    /// again. After that you probably want to re-link returned handle with its previous
    /// parent.
    #[inline]
    pub fn put_sub_graph_back(&mut self, sub_graph: SubGraph) -> Handle<Node> {
        for (ticket, node) in sub_graph.descendants {
            self.pool.put_back(ticket, node);
        }

        let (ticket, node) = sub_graph.root;
        let root_handle = self.put_back(ticket, node);

        self.link_nodes(root_handle, sub_graph.parent);

        root_handle
    }

    /// Forgets the entire sub-graph making handles to nodes invalid.
    #[inline]
    pub fn forget_sub_graph(&mut self, sub_graph: SubGraph) {
        for (ticket, _) in sub_graph.descendants {
            self.pool.forget_ticket(ticket);
        }
        let (ticket, _) = sub_graph.root;
        self.pool.forget_ticket(ticket);
    }

    /// Returns the number of nodes in the graph.
    #[inline]
    pub fn node_count(&self) -> u32 {
        self.pool.alive_count()
    }

    /// Creates deep copy of graph. Allows filtering while copying, returns copy and
    /// old-to-new node mapping.
    #[inline]
    pub fn clone<F, Pre, Post>(
        &self,
        root: Handle<Node>,
        filter: &mut F,
        pre_process_callback: &mut Pre,
        post_process_callback: &mut Post,
    ) -> (Self, NodeHandleMap<Node>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
        Pre: FnMut(Handle<Node>, &mut Node),
        Post: FnMut(Handle<Node>, Handle<Node>, &mut Node),
    {
        let mut copy = Self {
            sound_context: self.sound_context.deep_clone(),
            physics: self.physics.clone(),
            physics2d: self.physics2d.clone(),
            ..Default::default()
        };

        let (copy_root, old_new_map) = self.copy_node(
            root,
            &mut copy,
            filter,
            pre_process_callback,
            post_process_callback,
        );
        assert_eq!(copy.root, copy_root);

        let mut lightmap = self.lightmap.clone();
        if let Some(lightmap) = lightmap.as_mut() {
            let mut map = FxHashMap::default();
            for (mut handle, mut entries) in std::mem::take(&mut lightmap.map) {
                for entry in entries.iter_mut() {
                    for light_handle in entry.lights.iter_mut() {
                        old_new_map.try_map(light_handle);
                    }
                }

                if old_new_map.try_map(&mut handle) {
                    map.insert(handle, entries);
                }
            }
            lightmap.map = map;
        }
        copy.lightmap = lightmap;

        (copy, old_new_map)
    }

    /// Returns local transformation matrix of a node without scale.
    #[inline]
    pub fn local_transform_no_scale(&self, node: Handle<Node>) -> Matrix4<f32> {
        let mut transform = self[node].local_transform().clone();
        transform.set_scale(Vector3::new(1.0, 1.0, 1.0));
        transform.matrix()
    }

    /// Returns world transformation matrix of a node without scale.
    #[inline]
    pub fn global_transform_no_scale(&self, node: Handle<Node>) -> Matrix4<f32> {
        let parent = self[node].parent();
        if parent.is_some() {
            self.global_transform_no_scale(parent) * self.local_transform_no_scale(node)
        } else {
            self.local_transform_no_scale(node)
        }
    }

    /// Returns isometric local transformation matrix of a node. Such transform has
    /// only translation and rotation.
    #[inline]
    pub fn isometric_local_transform(&self, node: Handle<Node>) -> Matrix4<f32> {
        isometric_local_transform(&self.pool, node)
    }

    /// Returns world transformation matrix of a node only.  Such transform has
    /// only translation and rotation.
    #[inline]
    pub fn isometric_global_transform(&self, node: Handle<Node>) -> Matrix4<f32> {
        isometric_global_transform(&self.pool, node)
    }

    /// Returns global scale matrix of a node.
    #[inline]
    pub fn global_scale_matrix(&self, node: Handle<Node>) -> Matrix4<f32> {
        Matrix4::new_nonuniform_scaling(&self.global_scale(node))
    }

    /// Returns rotation quaternion of a node in world coordinates.
    #[inline]
    pub fn global_rotation(&self, node: Handle<Node>) -> UnitQuaternion<f32> {
        UnitQuaternion::from(Rotation3::from_matrix_eps(
            &self.global_transform_no_scale(node).basis(),
            f32::EPSILON,
            16,
            Rotation3::identity(),
        ))
    }

    /// Returns rotation quaternion of a node in world coordinates without pre- and post-rotations.
    #[inline]
    pub fn isometric_global_rotation(&self, node: Handle<Node>) -> UnitQuaternion<f32> {
        UnitQuaternion::from(Rotation3::from_matrix_eps(
            &self.isometric_global_transform(node).basis(),
            f32::EPSILON,
            16,
            Rotation3::identity(),
        ))
    }

    /// Returns rotation quaternion and position of a node in world coordinates, scale is eliminated.
    #[inline]
    pub fn global_rotation_position_no_scale(
        &self,
        node: Handle<Node>,
    ) -> (UnitQuaternion<f32>, Vector3<f32>) {
        (self.global_rotation(node), self[node].global_position())
    }

    /// Returns isometric global rotation and position.
    #[inline]
    pub fn isometric_global_rotation_position(
        &self,
        node: Handle<Node>,
    ) -> (UnitQuaternion<f32>, Vector3<f32>) {
        (
            self.isometric_global_rotation(node),
            self[node].global_position(),
        )
    }

    /// Returns global scale of a node.
    #[inline]
    pub fn global_scale(&self, mut node: Handle<Node>) -> Vector3<f32> {
        let mut global_scale = Vector3::repeat(1.0);
        while let Some(node_ref) = self.try_get(node) {
            global_scale = global_scale.component_mul(node_ref.local_transform().scale());
            node = node_ref.parent;
        }
        global_scale
    }

    /// Tries to borrow a node using the given handle and searches the script buffer for a script
    /// of type T and cast the first script, that could be found to the specified type.
    #[inline]
    pub fn try_get_script_of<T>(&self, node: Handle<Node>) -> Option<&T>
    where
        T: ScriptTrait,
    {
        self.try_get(node)
            .and_then(|node| node.try_get_script::<T>())
    }

    /// Tries to borrow a node and query all scripts of the given type `T`. This method returns
    /// [`None`] if the given node handle is invalid, otherwise it returns an iterator over the
    /// scripts of the type `T`.
    #[inline]
    pub fn try_get_scripts_of<T: ScriptTrait>(
        &self,
        node: Handle<Node>,
    ) -> Option<impl Iterator<Item = &T>> {
        self.try_get(node).map(|n| n.try_get_scripts())
    }

    /// Tries to borrow a node using the given handle and searches the script buffer for a script
    /// of type T and cast the first script, that could be found to the specified type.
    #[inline]
    pub fn try_get_script_of_mut<T>(&mut self, node: Handle<Node>) -> Option<&mut T>
    where
        T: ScriptTrait,
    {
        self.try_get_mut(node)
            .and_then(|node| node.try_get_script_mut::<T>())
    }

    /// Tries to borrow a node and query all scripts of the given type `T`. This method returns
    /// [`None`] if the given node handle is invalid, otherwise it returns an iterator over the
    /// scripts of the type `T`.
    #[inline]
    pub fn try_get_scripts_of_mut<T: ScriptTrait>(
        &mut self,
        node: Handle<Node>,
    ) -> Option<impl Iterator<Item = &mut T>> {
        self.try_get_mut(node).map(|n| n.try_get_scripts_mut())
    }

    /// Tries to borrow a node and find a component of the given type `C` across **all** available
    /// scripts of the node. If you want to search a component `C` in a particular script, then use
    /// [`Self::try_get_script_of`] and then search for component in it.
    #[inline]
    pub fn try_get_script_component_of<C>(&self, node: Handle<Node>) -> Option<&C>
    where
        C: Any,
    {
        self.try_get(node)
            .and_then(|node| node.try_get_script_component())
    }

    /// Tries to borrow a node and find a component of the given type `C` across **all** available
    /// scripts of the node. If you want to search a component `C` in a particular script, then use
    /// [`Self::try_get_script_of_mut`] and then search for component in it.
    #[inline]
    pub fn try_get_script_component_of_mut<C>(&mut self, node: Handle<Node>) -> Option<&mut C>
    where
        C: Any,
    {
        self.try_get_mut(node)
            .and_then(|node| node.try_get_script_component_mut())
    }

    /// Returns a handle of the node that has the given id.
    pub fn id_to_node_handle(&self, id: SceneNodeId) -> Option<&Handle<Node>> {
        self.instance_id_map.get(&id)
    }

    /// Tries to borrow a node by its id.
    pub fn node_by_id(&self, id: SceneNodeId) -> Option<(Handle<Node>, &Node)> {
        self.instance_id_map
            .get(&id)
            .and_then(|h| self.pool.try_borrow(*h).map(|n| (*h, n)))
    }

    /// Tries to borrow a node by its id.
    pub fn node_by_id_mut(&mut self, id: SceneNodeId) -> Option<(Handle<Node>, &mut Node)> {
        self.instance_id_map
            .get(&id)
            .and_then(|h| self.pool.try_borrow_mut(*h).map(|n| (*h, n)))
    }
}

impl<T, B: BorrowAs<Node, NodeContainer, Target = T>> Index<B> for Graph {
    type Output = T;

    #[inline]
    fn index(&self, typed_handle: B) -> &Self::Output {
        self.typed_ref(typed_handle)
            .expect("The node handle is invalid or the object it points to has different type.")
    }
}

impl<T, B: BorrowAs<Node, NodeContainer, Target = T>> IndexMut<B> for Graph {
    #[inline]
    fn index_mut(&mut self, typed_handle: B) -> &mut Self::Output {
        self.typed_mut(typed_handle)
            .expect("The node handle is invalid or the object it points to has different type.")
    }
}

impl Visit for Graph {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        // Pool must be empty, otherwise handles will be invalid and everything will blow up.
        if visitor.is_reading() && self.pool.get_capacity() != 0 {
            panic!("Graph pool must be empty on load!")
        }

        let mut region = visitor.enter_region(name)?;

        self.root.visit("Root", &mut region)?;
        self.pool.visit("Pool", &mut region)?;
        self.sound_context.visit("SoundContext", &mut region)?;
        self.physics.visit("PhysicsWorld", &mut region)?;
        self.physics2d.visit("PhysicsWorld2D", &mut region)?;
        let _ = self.lightmap.visit("Lightmap", &mut region);

        Ok(())
    }
}

impl AbstractSceneGraph for Graph {
    fn try_get_node_untyped(&self, handle: ErasedHandle) -> Option<&dyn AbstractSceneNode> {
        self.pool
            .try_borrow(handle.into())
            .map(|n| n as &dyn AbstractSceneNode)
    }

    fn try_get_node_untyped_mut(
        &mut self,
        handle: ErasedHandle,
    ) -> Option<&mut dyn AbstractSceneNode> {
        self.pool
            .try_borrow_mut(handle.into())
            .map(|n| n as &mut dyn AbstractSceneNode)
    }
}

impl BaseSceneGraph for Graph {
    type Prefab = Model;
    type NodeContainer = NodeContainer;
    type Node = Node;

    #[inline]
    fn actual_type_id(&self, handle: Handle<Self::Node>) -> Option<TypeId> {
        self.pool
            .try_borrow(handle)
            .map(|n| NodeAsAny::as_any(n.0.deref()).type_id())
    }

    #[inline]
    fn root(&self) -> Handle<Self::Node> {
        self.root
    }

    #[inline]
    fn set_root(&mut self, root: Handle<Self::Node>) {
        self.root = root;
    }

    #[inline]
    fn is_valid_handle(&self, handle: Handle<Self::Node>) -> bool {
        self.pool.is_valid_handle(handle)
    }

    #[inline]
    fn add_node(&mut self, mut node: Self::Node) -> Handle<Self::Node> {
        let children = node.children.clone();
        node.children.clear();
        let script_count = node.scripts.len();
        let handle = self.pool.spawn(node);

        if self.root.is_none() {
            self.root = handle;
        } else {
            self.link_nodes(handle, self.root);
        }

        for child in children {
            self.link_nodes(child, handle);
        }

        self.event_broadcaster.broadcast(GraphEvent::Added(handle));
        for i in 0..script_count {
            self.script_message_sender
                .send(NodeScriptMessage::InitializeScript {
                    handle,
                    script_index: i,
                })
                .unwrap();
        }

        let script_message_sender = self.script_message_sender.clone();
        let message_sender = self.message_sender.clone();
        let node = &mut self.pool[handle];
        node.on_connected_to_graph(handle, message_sender, script_message_sender);

        self.instance_id_map.insert(node.instance_id, handle);

        handle
    }

    #[inline]
    fn remove_node(&mut self, node_handle: Handle<Self::Node>) {
        self.isolate_node(node_handle);

        self.stack.clear();
        self.stack.push(node_handle);
        while let Some(handle) = self.stack.pop() {
            for &child in self.pool[handle].children().iter() {
                self.stack.push(child);
            }

            // Remove associated entities.
            let mut node = self.pool.free(handle);
            self.instance_id_map.remove(&node.instance_id);
            node.on_removed_from_graph(self);

            self.event_broadcaster
                .broadcast(GraphEvent::Removed(handle));
        }
    }

    #[inline]
    fn link_nodes(&mut self, child: Handle<Self::Node>, parent: Handle<Self::Node>) {
        self.isolate_node(child);
        self.pool[child].parent = parent;
        self.pool[parent].children.push(child);

        // Force update of global transform of the node being attached.
        self.message_sender
            .send(NodeMessage::new(child, NodeMessageKind::TransformChanged))
            .unwrap();
    }

    #[inline]
    fn unlink_node(&mut self, node_handle: Handle<Node>) {
        self.isolate_node(node_handle);
        self.link_nodes(node_handle, self.root);
        self.pool[node_handle]
            .local_transform_mut()
            .set_position(Vector3::default());
    }

    #[inline]
    fn isolate_node(&mut self, node_handle: Handle<Self::Node>) {
        // Replace parent handle of child
        let parent_handle = std::mem::replace(&mut self.pool[node_handle].parent, Handle::NONE);

        // Remove child from parent's children list
        if let Some(parent) = self.pool.try_borrow_mut(parent_handle) {
            if let Some(i) = parent.children().iter().position(|h| *h == node_handle) {
                parent.children.remove(i);
            }
        }

        let (ticket, mut node) = self.pool.take_reserve(node_handle);
        node.on_unlink(self);
        self.pool.put_back(ticket, node);
    }

    #[inline]
    fn try_get(&self, handle: Handle<Self::Node>) -> Option<&Self::Node> {
        self.pool.try_borrow(handle)
    }

    #[inline]
    fn try_get_mut(&mut self, handle: Handle<Self::Node>) -> Option<&mut Self::Node> {
        self.pool.try_borrow_mut(handle)
    }

    fn derived_type_ids(&self, handle: Handle<Self::Node>) -> Option<Vec<TypeId>> {
        self.pool
            .try_borrow(handle)
            .map(|n| Box::deref(&n.0).query_derived_types().to_vec())
    }

    fn actual_type_name(&self, handle: Handle<Self::Node>) -> Option<&'static str> {
        self.pool
            .try_borrow(handle)
            .map(|n| n.0.deref().type_name())
    }
}

impl SceneGraph for Graph {
    #[inline]
    fn pair_iter(&self) -> impl Iterator<Item = (Handle<Self::Node>, &Self::Node)> {
        self.pool.pair_iter()
    }

    #[inline]
    fn linear_iter(&self) -> impl Iterator<Item = &Self::Node> {
        self.pool.iter()
    }

    #[inline]
    fn linear_iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Node> {
        self.pool.iter_mut()
    }

    fn typed_ref<Ref>(
        &self,
        handle: impl BorrowAs<Self::Node, Self::NodeContainer, Target = Ref>,
    ) -> Option<&Ref> {
        self.pool.typed_ref(handle)
    }

    fn typed_mut<Ref>(
        &mut self,
        handle: impl BorrowAs<Self::Node, Self::NodeContainer, Target = Ref>,
    ) -> Option<&mut Ref> {
        self.pool.typed_mut(handle)
    }
}

#[cfg(test)]
mod test {
    use crate::scene::rigidbody::{RigidBody, RigidBodyBuilder};
    use crate::{
        asset::{io::FsResourceIo, manager::ResourceManager},
        core::{
            algebra::{Matrix4, Vector3},
            futures::executor::block_on,
            pool::Handle,
            reflect::prelude::*,
            type_traits::prelude::*,
            visitor::prelude::*,
        },
        engine::{self, SerializationContext},
        graph::{BaseSceneGraph, SceneGraph},
        resource::model::{Model, ModelResourceExtension},
        scene::{
            base::BaseBuilder,
            graph::Graph,
            mesh::{
                surface::{SurfaceBuilder, SurfaceData, SurfaceResource},
                MeshBuilder,
            },
            node::Node,
            pivot::{Pivot, PivotBuilder},
            transform::TransformBuilder,
            Scene, SceneLoader,
        },
        script::ScriptTrait,
    };
    use fyrox_core::algebra::Vector2;
    use fyrox_core::append_extension;
    use fyrox_resource::untyped::ResourceKind;
    use std::{fs, path::Path, sync::Arc};

    #[derive(Clone, Debug, PartialEq, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "722feb80-a10b-4ee0-8cef-5d1473df8457")]
    struct MyScript {
        foo: String,
        bar: f32,
    }

    impl ScriptTrait for MyScript {}

    #[derive(Clone, Debug, PartialEq, Reflect, Visit, TypeUuidProvider, ComponentProvider)]
    #[type_uuid(id = "722feb80-a10b-4ee0-8cef-5d1473df8458")]
    struct MyOtherScript {
        baz: u32,
        foobar: Vec<u32>,
    }

    impl ScriptTrait for MyOtherScript {}

    #[test]
    fn test_graph_scripts() {
        let node = PivotBuilder::new(
            BaseBuilder::new()
                .with_script(MyScript {
                    foo: "Stuff".to_string(),
                    bar: 123.321,
                })
                .with_script(MyScript {
                    foo: "OtherStuff".to_string(),
                    bar: 321.123,
                })
                .with_script(MyOtherScript {
                    baz: 321,
                    foobar: vec![1, 2, 3],
                }),
        )
        .build_node();

        let mut graph = Graph::new();

        let handle = graph.add_node(node);

        assert_eq!(
            graph.try_get_script_of::<MyScript>(handle),
            Some(&MyScript {
                foo: "Stuff".to_string(),
                bar: 123.321,
            })
        );
        assert_eq!(
            graph.try_get_script_of_mut::<MyScript>(handle),
            Some(&mut MyScript {
                foo: "Stuff".to_string(),
                bar: 123.321,
            })
        );

        let mut immutable_iterator = graph
            .try_get_scripts_of::<MyScript>(handle)
            .expect("The handle expected to be valid!");
        assert_eq!(
            immutable_iterator.next(),
            Some(&MyScript {
                foo: "Stuff".to_string(),
                bar: 123.321,
            })
        );
        assert_eq!(
            immutable_iterator.next(),
            Some(&MyScript {
                foo: "OtherStuff".to_string(),
                bar: 321.123,
            })
        );
        drop(immutable_iterator);

        assert_eq!(
            graph.try_get_script_of::<MyOtherScript>(handle),
            Some(&MyOtherScript {
                baz: 321,
                foobar: vec![1, 2, 3],
            })
        );
        assert_eq!(
            graph.try_get_script_of_mut::<MyOtherScript>(handle),
            Some(&mut MyOtherScript {
                baz: 321,
                foobar: vec![1, 2, 3],
            })
        );

        let mut mutable_iterator = graph
            .try_get_scripts_of_mut::<MyScript>(handle)
            .expect("The handle expected to be valid!");
        assert_eq!(
            mutable_iterator.next(),
            Some(&mut MyScript {
                foo: "Stuff".to_string(),
                bar: 123.321,
            })
        );
        assert_eq!(
            mutable_iterator.next(),
            Some(&mut MyScript {
                foo: "OtherStuff".to_string(),
                bar: 321.123,
            })
        );
    }

    #[test]
    fn graph_init_test() {
        let graph = Graph::new();
        assert_ne!(graph.root, Handle::NONE);
        assert_eq!(graph.pool.alive_count(), 1);
    }

    #[test]
    fn graph_node_test() {
        let mut graph = Graph::new();
        graph.add_node(Node::new(Pivot::default()));
        graph.add_node(Node::new(Pivot::default()));
        graph.add_node(Node::new(Pivot::default()));
        assert_eq!(graph.pool.alive_count(), 4);
    }

    #[test]
    fn test_graph_search() {
        let mut graph = Graph::new();

        // Root_
        //      |_A_
        //          |_B
        //          |_C_
        //             |_D
        let b;
        let c;
        let d;
        let a = PivotBuilder::new(BaseBuilder::new().with_name("A").with_children(&[
            {
                b = PivotBuilder::new(BaseBuilder::new().with_name("B")).build(&mut graph);
                b
            },
            {
                c = PivotBuilder::new(BaseBuilder::new().with_name("C").with_children(&[{
                    d = PivotBuilder::new(BaseBuilder::new().with_name("D")).build(&mut graph);
                    d
                }]))
                .build(&mut graph);
                c
            },
        ]))
        .build(&mut graph);

        // Test down search.
        assert!(graph.find_by_name(a, "X").is_none());
        assert_eq!(graph.find_by_name(a, "A").unwrap().0, a);
        assert_eq!(graph.find_by_name(a, "D").unwrap().0, d);

        let result = graph
            .find_map(a, &mut |n| if n.name() == "D" { Some("D") } else { None })
            .unwrap();
        assert_eq!(result.0, d);
        assert_eq!(result.1, "D");

        // Test up search.
        assert!(graph.find_up_by_name(d, "X").is_none());
        assert_eq!(graph.find_up_by_name(d, "D").unwrap().0, d);
        assert_eq!(graph.find_up_by_name(d, "A").unwrap().0, a);

        let result = graph
            .find_up_map(d, &mut |n| if n.name() == "A" { Some("A") } else { None })
            .unwrap();
        assert_eq!(result.0, a);
        assert_eq!(result.1, "A");
    }

    fn create_scene() -> Scene {
        let mut scene = Scene::new();

        PivotBuilder::new(BaseBuilder::new().with_name("Pivot")).build(&mut scene.graph);

        PivotBuilder::new(BaseBuilder::new().with_name("MeshPivot").with_children(&[{
            MeshBuilder::new(
                BaseBuilder::new().with_name("Mesh").with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(3.0, 2.0, 1.0))
                        .build(),
                ),
            )
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceResource::new_ok(
                Uuid::new_v4(),
                ResourceKind::Embedded,
                SurfaceData::make_cone(16, 1.0, 1.0, &Matrix4::identity()),
            ))
            .build()])
            .build(&mut scene.graph)
        }]))
        .build(&mut scene.graph);

        scene
    }

    fn save_scene(scene: &mut Scene, path: &Path) {
        let mut visitor = Visitor::new();
        scene.save("Scene", &mut visitor).unwrap();
        visitor.save_binary(path).unwrap();
        visitor
            .save_text_to_file(append_extension(path, "txt"))
            .unwrap();
    }

    fn make_resource_manager() -> ResourceManager {
        let resource_manager = ResourceManager::new(Arc::new(Default::default()));
        engine::initialize_resource_manager_loaders(
            &resource_manager,
            Arc::new(SerializationContext::new()),
        );
        resource_manager.update_and_load_registry("test_output/resources.registry");
        resource_manager
    }

    #[test]
    fn test_restore_integrity() {
        if !Path::new("test_output").exists() {
            fs::create_dir_all("test_output").unwrap();
        }

        let root_asset_path = Path::new("test_output/root2.rgs");
        let derived_asset_path = Path::new("test_output/derived2.rgs");

        // Create root scene and save it.
        {
            let mut scene = create_scene();
            save_scene(&mut scene, root_asset_path);
        }

        // Create root resource instance in a derived resource. This creates a derived asset.
        {
            let resource_manager = make_resource_manager();
            let root_asset = block_on(resource_manager.request::<Model>(root_asset_path)).unwrap();

            let mut derived = Scene::new();
            root_asset.instantiate(&mut derived);
            save_scene(&mut derived, derived_asset_path);
        }

        // Now load the root asset, modify it, save it back and reload the derived asset.
        {
            let resource_manager = make_resource_manager();
            let mut scene = block_on(
                block_on(SceneLoader::from_file(
                    root_asset_path,
                    &FsResourceIo,
                    Arc::new(SerializationContext::new()),
                    resource_manager.clone(),
                ))
                .unwrap()
                .0
                .finish(),
            );

            // Add a new node to the root node of the scene.
            PivotBuilder::new(BaseBuilder::new().with_name("AddedLater")).build(&mut scene.graph);

            // Add a new node to the mesh.
            let mesh = scene.graph.find_by_name_from_root("Mesh").unwrap().0;
            let pivot = PivotBuilder::new(BaseBuilder::new().with_name("NewChildOfMesh"))
                .build(&mut scene.graph);
            scene.graph.link_nodes(pivot, mesh);

            // Remove existing nodes.
            let existing_pivot = scene.graph.find_by_name_from_root("Pivot").unwrap().0;
            scene.graph.remove_node(existing_pivot);

            // Save the scene back.
            save_scene(&mut scene, root_asset_path);
        }

        // Load the derived scene and check if its content was synced with the content of the root asset.
        {
            let resource_manager = make_resource_manager();
            let derived_asset =
                block_on(resource_manager.request::<Model>(derived_asset_path)).unwrap();

            let derived_data = derived_asset.data_ref();
            let derived_scene = derived_data.get_scene();

            // Pivot must also be removed from the derived asset, because it is deleted in the root asset.
            assert_eq!(
                derived_scene
                    .graph
                    .find_by_name_from_root("Pivot")
                    .map(|(h, _)| h),
                None
            );

            let mesh_pivot = derived_scene
                .graph
                .find_by_name_from_root("MeshPivot")
                .unwrap()
                .0;
            let mesh = derived_scene
                .graph
                .find_by_name(mesh_pivot, "Mesh")
                .unwrap()
                .0;
            derived_scene
                .graph
                .find_by_name_from_root("AddedLater")
                .unwrap();
            derived_scene
                .graph
                .find_by_name(mesh, "NewChildOfMesh")
                .unwrap();
        }
    }

    #[test]
    fn test_global_scale() {
        let mut graph = Graph::new();

        let b;
        let c;
        let a = PivotBuilder::new(
            BaseBuilder::new()
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_scale(Vector3::new(1.0, 1.0, 2.0))
                        .build(),
                )
                .with_children(&[{
                    b = PivotBuilder::new(
                        BaseBuilder::new()
                            .with_local_transform(
                                TransformBuilder::new()
                                    .with_local_scale(Vector3::new(3.0, 2.0, 1.0))
                                    .build(),
                            )
                            .with_children(&[{
                                c = PivotBuilder::new(
                                    BaseBuilder::new().with_local_transform(
                                        TransformBuilder::new()
                                            .with_local_scale(Vector3::new(1.0, 2.0, 3.0))
                                            .build(),
                                    ),
                                )
                                .build(&mut graph);
                                c
                            }]),
                    )
                    .build(&mut graph);
                    b
                }]),
        )
        .build(&mut graph);

        assert_eq!(graph.global_scale(a), Vector3::new(1.0, 1.0, 2.0));
        assert_eq!(graph.global_scale(b), Vector3::new(3.0, 2.0, 2.0));
        assert_eq!(graph.global_scale(c), Vector3::new(3.0, 4.0, 6.0));
    }

    #[test]
    fn test_hierarchy_changes_propagation() {
        let mut graph = Graph::new();

        let b;
        let c;
        let d;
        let a = PivotBuilder::new(
            BaseBuilder::new()
                .with_local_transform(
                    TransformBuilder::new()
                        .with_local_position(Vector3::new(1.0, 0.0, 0.0))
                        .build(),
                )
                .with_children(&[
                    {
                        b = PivotBuilder::new(
                            BaseBuilder::new()
                                .with_visibility(false)
                                .with_enabled(false)
                                .with_local_transform(
                                    TransformBuilder::new()
                                        .with_local_position(Vector3::new(0.0, 1.0, 0.0))
                                        .build(),
                                )
                                .with_children(&[{
                                    c = PivotBuilder::new(
                                        BaseBuilder::new().with_local_transform(
                                            TransformBuilder::new()
                                                .with_local_position(Vector3::new(0.0, 0.0, 1.0))
                                                .build(),
                                        ),
                                    )
                                    .build(&mut graph);
                                    c
                                }]),
                        )
                        .build(&mut graph);
                        b
                    },
                    {
                        d = PivotBuilder::new(
                            BaseBuilder::new().with_local_transform(
                                TransformBuilder::new()
                                    .with_local_position(Vector3::new(1.0, 1.0, 1.0))
                                    .build(),
                            ),
                        )
                        .build(&mut graph);
                        d
                    },
                ]),
        )
        .build(&mut graph);

        graph.update(Vector2::new(1.0, 1.0), 1.0 / 60.0, Default::default());

        assert_eq!(graph[a].global_position(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(graph[b].global_position(), Vector3::new(1.0, 1.0, 0.0));
        assert_eq!(graph[c].global_position(), Vector3::new(1.0, 1.0, 1.0));
        assert_eq!(graph[d].global_position(), Vector3::new(2.0, 1.0, 1.0));

        assert!(graph[a].global_visibility());
        assert!(!graph[b].global_visibility());
        assert!(!graph[c].global_visibility());
        assert!(graph[d].global_visibility());

        assert!(graph[a].is_globally_enabled());
        assert!(!graph[b].is_globally_enabled());
        assert!(!graph[c].is_globally_enabled());
        assert!(graph[d].is_globally_enabled());

        // Change something
        graph[b]
            .local_transform_mut()
            .set_position(Vector3::new(0.0, 2.0, 0.0));
        graph[a].set_enabled(false);
        graph[b].set_visibility(true);

        graph.update(Vector2::new(1.0, 1.0), 1.0 / 60.0, Default::default());

        assert_eq!(graph[a].global_position(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(graph[b].global_position(), Vector3::new(1.0, 2.0, 0.0));
        assert_eq!(graph[c].global_position(), Vector3::new(1.0, 2.0, 1.0));
        assert_eq!(graph[d].global_position(), Vector3::new(2.0, 1.0, 1.0));

        assert!(graph[a].global_visibility());
        assert!(graph[b].global_visibility());
        assert!(graph[c].global_visibility());
        assert!(graph[d].global_visibility());

        assert!(!graph.pool.typed_ref(a).unwrap().is_globally_enabled());
        assert!(!graph[b].is_globally_enabled());
        assert!(!graph[c].is_globally_enabled());
        assert!(!graph[d].is_globally_enabled());
    }

    #[test]
    fn test_typed_borrow() {
        let mut graph = Graph::new();
        let pivot = PivotBuilder::new(BaseBuilder::new()).build(&mut graph);
        let rigid_body = RigidBodyBuilder::new(BaseBuilder::new()).build(&mut graph);

        assert!(graph.pool.typed_ref(pivot).is_some());
        assert!(graph.pool.typed_ref(pivot.transmute::<Pivot>()).is_some());
        assert!(graph
            .pool
            .typed_ref(pivot.transmute::<RigidBody>())
            .is_none());

        assert!(graph.pool.typed_ref(rigid_body).is_some());
        assert!(graph
            .pool
            .typed_ref(rigid_body.transmute::<RigidBody>())
            .is_some());
        assert!(graph
            .pool
            .typed_ref(rigid_body.transmute::<Pivot>())
            .is_none());
    }
}
