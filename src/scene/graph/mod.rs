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

use crate::scene::node::GenericContext;
use crate::{
    asset::{manager::ResourceManager, untyped::UntypedResource},
    core::{
        algebra::{Matrix4, Rotation3, UnitQuaternion, Vector2, Vector3},
        instant,
        log::{Log, MessageKind},
        math::{aabb::AxisAlignedBoundingBox, Matrix4Ext},
        pool::{Handle, MultiBorrowContext, Ticket},
        reflect::prelude::*,
        sstorage::ImmutableString,
        variable::{self, try_inherit_properties},
        visitor::{Visit, VisitResult, Visitor},
    },
    material::{shader::SamplerFallback, MaterialResource, PropertyValue},
    resource::model::{ModelResource, ModelResourceExtension, NodeMapping},
    scene::{
        base::{NodeScriptMessage, SceneNodeId},
        camera::Camera,
        dim2::{self},
        graph::{
            event::{GraphEvent, GraphEventBroadcaster},
            map::NodeHandleMap,
            physics::{PhysicsPerformanceStatistics, PhysicsWorld},
        },
        mesh::buffer::{
            VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage,
            VertexWriteTrait,
        },
        mesh::Mesh,
        navmesh,
        node::{container::NodeContainer, Node, NodeTrait, SyncContext, UpdateContext},
        pivot::Pivot,
        sound::context::SoundContext,
        transform::TransformBuilder,
    },
    script::ScriptTrait,
    utils::lightmap::Lightmap,
};
use fxhash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    ops::{Index, IndexMut},
    sync::mpsc::{channel, Receiver, Sender},
    time::Duration,
};

pub mod event;
pub mod map;
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
pub type LowLevelGraph = fyrox_graph::Graph<Node, NodeContainer>;

/// See module docs.
#[derive(Debug, Reflect)]
pub struct Graph {
    inner: LowLevelGraph,

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

    instance_id_map: FxHashMap<SceneNodeId, Handle<Node>>,
}

impl Default for Graph {
    fn default() -> Self {
        let (tx, rx) = channel();

        Self {
            physics: PhysicsWorld::new(),
            physics2d: dim2::physics::PhysicsWorld::new(),
            inner: LowLevelGraph::new(),
            sound_context: Default::default(),
            performance_statistics: Default::default(),
            event_broadcaster: Default::default(),
            script_message_receiver: rx,
            script_message_sender: tx,
            lightmap: None,
            instance_id_map: Default::default(),
        }
    }
}

/// Sub-graph is a piece of graph that was extracted from a graph. It has ownership
/// over its nodes. It is used to temporarily take ownership of a sub-graph. This could
/// be used if you making a scene editor with a command stack - once you reverted a command,
/// that created a complex nodes hierarchy (for example you loaded a model) you must store
/// all added nodes somewhere to be able put nodes back into graph when user decide to re-do
/// command. Sub-graph allows you to do this without invalidating handles to nodes.
pub type SubGraph = fyrox_graph::SubGraph<Node>;

fn remap_handles(old_new_mapping: &NodeHandleMap, dest_graph: &mut Graph) {
    // Iterate over instantiated nodes and remap handles.
    for (_, &new_node_handle) in old_new_mapping.inner().iter() {
        old_new_mapping.remap_handles(
            &mut dest_graph.inner[new_node_handle],
            &[TypeId::of::<UntypedResource>()],
        );
    }
}

fn isometric_local_transform(nodes: &LowLevelGraph, node: Handle<Node>) -> Matrix4<f32> {
    let transform = nodes[node].local_transform();
    TransformBuilder::new()
        .with_local_position(**transform.position())
        .with_local_rotation(**transform.rotation())
        .with_pre_rotation(**transform.pre_rotation())
        .with_post_rotation(**transform.post_rotation())
        .build()
        .matrix()
}

fn isometric_global_transform(nodes: &LowLevelGraph, node: Handle<Node>) -> Matrix4<f32> {
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
    node.base_node = Default::default();
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
        let (tx, rx) = channel();

        // Create root node.
        let mut root_node = Pivot::default();
        let instance_id = root_node.instance_id;
        root_node.script_message_sender = Some(tx.clone());
        root_node.set_name("__ROOT__");

        // Add it to the pool.
        let mut inner = LowLevelGraph::new();
        let root = inner.add_node(Node::new(root_node));
        inner[root].self_handle = root;

        let instance_id_map = FxHashMap::from_iter([(instance_id, root)]);

        Self {
            physics: Default::default(),
            inner,
            physics2d: Default::default(),
            sound_context: SoundContext::new(),
            performance_statistics: Default::default(),
            event_broadcaster: Default::default(),
            script_message_receiver: rx,
            script_message_sender: tx,
            lightmap: None,
            instance_id_map,
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
    pub fn change_root(&mut self, root: Node) {
        let prev_root = self.inner.root;
        self.inner.root = Handle::NONE;
        let handle = self.add_node(root);
        assert_eq!(self.inner.root, handle);
        self.link_nodes(prev_root, handle);
    }

    /// Makes a node in the graph the new root of the graph. All children nodes of the previous root will
    /// become children nodes of the new root. Old root will become a child node of the new root.
    pub fn change_root_inplace(&mut self, new_root: Handle<Node>) {
        let prev_root = self.inner.root;
        self.unlink_internal(new_root);
        let prev_root_children = self
            .try_get(prev_root)
            .map(|r| r.base_node.children.clone())
            .unwrap_or_default();
        for child in prev_root_children {
            self.link_nodes(child, new_root);
        }
        if prev_root.is_some() {
            self.link_nodes(prev_root, new_root);
        }
        self.inner.root = new_root;
    }

    /// Adds new node to the graph. Node will be transferred into implementation-defined
    /// storage and you'll get a handle to the node. Node will be automatically attached
    /// to root node of graph, it is required because graph can contain only one root.
    #[inline]
    pub fn add_node(&mut self, node: Node) -> Handle<Node> {
        let has_script = node.script.is_some();
        let handle = self.inner.add_node(node);

        self.event_broadcaster.broadcast(GraphEvent::Added(handle));
        if has_script {
            self.script_message_sender
                .send(NodeScriptMessage::InitializeScript { handle })
                .unwrap();
        }

        let node = &mut self.inner[handle];
        node.self_handle = handle;
        node.script_message_sender = Some(self.script_message_sender.clone());

        self.instance_id_map.insert(node.instance_id, handle);

        handle
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

    /// Tries to borrow mutable references to two nodes at the same time by given handles. Will
    /// panic if handles overlaps (points to same node).
    #[inline]
    pub fn get_two_mut(&mut self, nodes: (Handle<Node>, Handle<Node>)) -> (&mut Node, &mut Node) {
        self.inner.borrow_two_mut(nodes)
    }

    /// Tries to borrow mutable references to three nodes at the same time by given handles. Will
    /// return Err of handles overlaps (points to same node).
    #[inline]
    pub fn get_three_mut(
        &mut self,
        nodes: (Handle<Node>, Handle<Node>, Handle<Node>),
    ) -> (&mut Node, &mut Node, &mut Node) {
        self.inner.borrow_three_mut(nodes)
    }

    /// Tries to borrow mutable references to four nodes at the same time by given handles. Will
    /// panic if handles overlaps (points to same node).
    #[inline]
    pub fn get_four_mut(
        &mut self,
        nodes: (Handle<Node>, Handle<Node>, Handle<Node>, Handle<Node>),
    ) -> (&mut Node, &mut Node, &mut Node, &mut Node) {
        self.inner.borrow_four_mut(nodes)
    }

    /// Returns root node of current graph.
    #[inline]
    pub fn get_root(&self) -> Handle<Node> {
        self.inner.root
    }

    /// Tries to borrow a node, returns Some(node) if the handle is valid, None - otherwise.
    #[inline]
    pub fn try_get(&self, handle: Handle<Node>) -> Option<&Node> {
        self.inner.try_borrow(handle)
    }

    /// Tries to borrow a node and fetch its component of specified type.
    #[inline]
    pub fn try_get_of_type<T>(&self, handle: Handle<Node>) -> Option<&T>
    where
        T: 'static,
    {
        self.try_get(handle)
            .and_then(|n| n.query_component_ref::<T>())
    }

    /// Tries to mutably borrow a node, returns Some(node) if the handle is valid, None - otherwise.
    #[inline]
    pub fn try_get_mut(&mut self, handle: Handle<Node>) -> Option<&mut Node> {
        self.inner.try_borrow_mut(handle)
    }

    /// Tries to mutably borrow a node and fetch its component of specified type.
    #[inline]
    pub fn try_get_mut_of_type<T>(&mut self, handle: Handle<Node>) -> Option<&mut T>
    where
        T: 'static,
    {
        self.try_get_mut(handle)
            .and_then(|n| n.query_component_mut::<T>())
    }

    /// Begins multi-borrow that allows you borrow to as many shared references to the graph
    /// nodes as you need and only one mutable reference to a node. See
    /// [`MultiBorrowContext::try_get`] for more info.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use fyrox::{
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
        self.inner.begin_multi_borrow()
    }

    fn on_node_removed(
        mbc: &MultiBorrowContext<Node, NodeContainer>,
        node: &mut Node,
        physics: &mut PhysicsWorld,
        physics2d: &mut dim2::physics::PhysicsWorld,
        sound_context: &mut SoundContext,
        instance_id_map: &mut FxHashMap<SceneNodeId, Handle<Node>>,
    ) {
        instance_id_map.remove(&node.instance_id);
        node.on_removed_from_graph(&mut GenericContext {
            nodes: &mbc,
            physics,
            physics2d,
            sound_context,
        });
    }

    /// Destroys the node and its children recursively. Scripts of the destroyed nodes will be removed in the next
    /// update tick.
    #[inline]
    pub fn remove_node(&mut self, node_handle: Handle<Node>) {
        self.unlink_internal(node_handle);

        self.inner
            .remove_node(node_handle, |handle, mut node, mbc| {
                Self::on_node_removed(
                    mbc,
                    &mut node,
                    &mut self.physics,
                    &mut self.physics2d,
                    &mut self.sound_context,
                    &mut self.instance_id_map,
                );

                self.event_broadcaster
                    .broadcast(GraphEvent::Removed(handle));
            });
    }

    fn unlink_internal(&mut self, node_handle: Handle<Node>) {
        self.inner.unlink(node_handle);

        let mbc = self.inner.begin_multi_borrow();
        let mut node = mbc.try_get_mut(node_handle).unwrap();
        node.on_unlink(&mut GenericContext {
            nodes: &mbc,
            physics: &mut self.physics,
            physics2d: &mut self.physics2d,
            sound_context: &mut self.sound_context,
        });
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child: Handle<Node>, parent: Handle<Node>) {
        self.unlink_internal(child);
        self.inner.link_nodes(child, parent, false);
    }

    /// Links specified child with specified parent while keeping the
    /// child's global position and rotation.
    #[inline]
    pub fn link_nodes_keep_global_position_rotation(
        &mut self,
        child: Handle<Node>,
        parent: Handle<Node>,
    ) {
        let parent_transform_inv = self.inner[parent]
            .global_transform()
            .try_inverse()
            .unwrap_or_default();
        let child_transform = self.inner[child].global_transform();
        let relative_transform = parent_transform_inv * child_transform;
        let local_position = relative_transform.position();
        let local_rotation = UnitQuaternion::from_matrix(&relative_transform.basis());
        self.inner[child]
            .local_transform_mut()
            .set_position(local_position)
            .set_rotation(local_rotation);
        self.link_nodes(child, parent);
    }

    /// Unlinks specified node from its parent and attaches it to root graph node.
    #[inline]
    pub fn unlink_node(&mut self, node_handle: Handle<Node>) {
        self.unlink_internal(node_handle);
        self.link_nodes(node_handle, self.inner.root);
        self.inner[node_handle]
            .local_transform_mut()
            .set_position(Vector3::default());
    }

    /// Tries to find a copy of `node_handle` in hierarchy tree starting from `root_handle`.
    #[inline]
    pub fn find_copy_of(
        &self,
        root_handle: Handle<Node>,
        node_handle: Handle<Node>,
    ) -> Handle<Node> {
        let root = &self.inner[root_handle];
        if root.original_handle_in_resource() == node_handle {
            return root_handle;
        }

        for child_handle in root.children() {
            let out = self.find_copy_of(*child_handle, node_handle);
            if out.is_some() {
                return out;
            }
        }

        Handle::NONE
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find(
        &self,
        root: Handle<Node>,
        cmp: impl FnMut(&Node) -> bool,
    ) -> Option<(Handle<Node>, &Node)> {
        self.inner.find(root, cmp)
    }

    /// Searches for a node down the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_map<T: ?Sized>(
        &self,
        root: Handle<Node>,
        cmp: impl FnMut(&Node) -> Option<&T>,
    ) -> Option<(Handle<Node>, &T)> {
        self.inner.find_map(root, cmp)
    }

    /// Searches for a node up the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_up(
        &self,
        root_node: Handle<Node>,
        cmp: impl FnMut(&Node) -> bool,
    ) -> Option<(Handle<Node>, &Node)> {
        self.inner.find_up(root_node, cmp)
    }

    /// Searches for a node up the tree starting from the specified node using the specified closure. Returns a tuple
    /// with a handle and a reference to the mapped value. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_up_map<T: ?Sized>(
        &self,
        root: Handle<Node>,
        cmp: impl FnMut(&Node) -> Option<&T>,
    ) -> Option<(Handle<Node>, &T)> {
        self.inner.find_up_map(root, cmp)
    }

    /// Searches for a node with the specified name down the tree starting from the specified node. Returns a tuple with
    /// a handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_by_name(&self, root: Handle<Node>, name: &str) -> Option<(Handle<Node>, &Node)> {
        self.inner.find_by_name(root, name)
    }

    /// Searches for a node with the specified name up the tree starting from the specified node. Returns a tuple with a
    /// handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_up_by_name(&self, root: Handle<Node>, name: &str) -> Option<(Handle<Node>, &Node)> {
        self.inner.find_up_by_name(root, name)
    }

    /// Searches for a node with the specified name down the tree starting from the graph root. Returns a tuple with a
    /// handle and a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_by_name_from_root(&self, name: &str) -> Option<(Handle<Node>, &Node)> {
        self.inner.find_by_name_from_root(name)
    }

    /// Searches for a **first** node with a script of the given type `S` in the hierarchy starting from the
    /// given `root_node`.
    #[inline]
    pub fn find_first_by_script<S>(&self, root_node: Handle<Node>) -> Option<(Handle<Node>, &Node)>
    where
        S: ScriptTrait,
    {
        self.find(root_node, |n| {
            n.script().and_then(|s| s.cast::<S>()).is_some()
        })
    }

    /// Searches node using specified compare closure starting from root. Returns a tuple with a handle and
    /// a reference to the found node. If nothing is found, it returns [`None`].
    #[inline]
    pub fn find_from_root(&self, cmp: impl FnMut(&Node) -> bool) -> Option<(Handle<Node>, &Node)> {
        self.inner.find_from_root(cmp)
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
    ) -> (Handle<Node>, NodeHandleMap)
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
    ) -> (Handle<Node>, NodeHandleMap)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let mut old_new_mapping = NodeHandleMap::default();

        let to_copy = self
            .traverse_handle_iter(node_handle)
            .map(|node| (node, self.inner[node].children().to_vec()))
            .collect::<Vec<_>>();

        let mut root_handle = Handle::NONE;

        for (parent, children) in to_copy.iter() {
            // Copy parent first.
            let parent_copy = clear_links(self.inner[*parent].clone_box());
            let parent_copy_handle = self.add_node(parent_copy);
            old_new_mapping.map.insert(*parent, parent_copy_handle);

            if root_handle.is_none() {
                root_handle = parent_copy_handle;
            }

            // Copy children and link to new parent.
            for &child in children {
                if filter(child, &self.inner[child]) {
                    let child_copy = clear_links(self.inner[child].clone_box());
                    let child_copy_handle = self.add_node(child_copy);
                    old_new_mapping.map.insert(child, child_copy_handle);
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
        let node = &self.inner[node_handle];
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
        old_new_mapping: &mut NodeHandleMap,
        filter: &mut F,
        pre_process_callback: &mut Pre,
        post_process_callback: &mut Post,
    ) -> Handle<Node>
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
        Pre: FnMut(Handle<Node>, &mut Node),
        Post: FnMut(Handle<Node>, Handle<Node>, &mut Node),
    {
        let src_node = &self.inner[root_handle];
        let mut dest_node = clear_links(src_node.clone_box());
        pre_process_callback(root_handle, &mut dest_node);
        let dest_copy_handle = dest_graph.add_node(dest_node);
        old_new_mapping.map.insert(root_handle, dest_copy_handle);
        for &src_child_handle in src_node.children() {
            if filter(src_child_handle, &self.inner[src_child_handle]) {
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

    fn restore_original_handles_and_inherit_properties(&mut self) {
        // Iterate over each node in the graph and resolve original handles. Original handle is a handle
        // to a node in resource from which a node was instantiated from. Also sync inheritable properties
        // if needed.
        for node in self.inner.iter_mut() {
            if let Some(model) = node.resource() {
                let mut header = model.state();
                let model_kind = header.kind().clone();
                if let Some(data) = header.data() {
                    let resource_graph = &data.get_scene().graph;

                    let resource_node = match data.mapping {
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
                                .inner
                                .try_borrow(node.original_handle_in_resource)
                                .map(|resource_node| {
                                    (resource_node, node.original_handle_in_resource)
                                })
                        }
                    };

                    if let Some((resource_node, original)) = resource_node {
                        node.original_handle_in_resource = original;
                        node.inv_bind_pose_transform = resource_node.inv_bind_pose_transform();

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
                                        node.self_handle.index(),
                                        node.self_handle.generation(),
                                        model_kind
                                    ));
                                }
                            })
                        });
                        if !types_match {
                            let base = (**node).clone();
                            let mut resource_node_clone = resource_node.clone_box();
                            variable::mark_inheritable_properties_non_modified(
                                &mut resource_node_clone as &mut dyn Reflect,
                                &[
                                    TypeId::of::<UntypedResource>(),
                                    TypeId::of::<navmesh::Container>(),
                                ],
                            );
                            **resource_node_clone = base;
                            *node = resource_node_clone;
                        }

                        // Then try to inherit properties.
                        node.as_reflect_mut(&mut |node_reflect| {
                            resource_node.as_reflect(&mut |resource_node_reflect| {
                                Log::verify(try_inherit_properties(
                                    node_reflect,
                                    resource_node_reflect,
                                    // Do not try to inspect resources, because it most likely cause a deadlock.
                                    &[
                                        std::any::TypeId::of::<UntypedResource>(),
                                        TypeId::of::<navmesh::Container>(),
                                    ],
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

    // Maps handles in properties of instances after property inheritance. It is needed, because when a
    // property contains node handle, the handle cannot be used directly after inheritance. Instead, it
    // must be mapped to respective instance first.
    //
    // To do so, we at first, build node handle mapping (original handle -> instance handle) starting from
    // instance root. Then we must find all inheritable properties and try to remap them to instance handles.
    fn remap_handles(&mut self, instances: &[(Handle<Node>, ModelResource)]) {
        for (instance_root, resource) in instances {
            // Prepare old -> new handle mapping first by walking over the graph
            // starting from instance root.
            let mut old_new_mapping = NodeHandleMap::default();
            let mut traverse_stack = vec![*instance_root];
            while let Some(node_handle) = traverse_stack.pop() {
                let node = &self.inner[node_handle];
                if let Some(node_resource) = node.resource().as_ref() {
                    // We're interested only in instance nodes.
                    if node_resource == resource {
                        let previous_mapping = old_new_mapping
                            .map
                            .insert(node.original_handle_in_resource, node_handle);
                        // There should be no such node.
                        if previous_mapping.is_some() {
                            Log::warn(format!(
                                "There are multiple original nodes for {:?}! Previous was {:?}. \
                                This can happen if a respective node was deleted.",
                                node_handle, node.original_handle_in_resource
                            ))
                        }
                    }
                }

                traverse_stack.extend_from_slice(node.children());
            }

            // Lastly, remap handles. We can't do this in single pass because there could
            // be cross references.
            for (_, handle) in old_new_mapping.map.iter() {
                old_new_mapping.remap_inheritable_handles(
                    &mut self.inner[*handle],
                    &[TypeId::of::<UntypedResource>()],
                );
            }
        }
    }

    // This method checks integrity of the graph and restores it if needed. For example, if a node was added in a parent asset,
    // then it must be added in the graph. Alternatively, if a node was deleted in a parent asset, then its instance must be
    // deleted in the graph.
    fn restore_integrity(&mut self) -> Vec<(Handle<Node>, ModelResource)> {
        Log::writeln(MessageKind::Information, "Checking integrity...");

        let instances = self
            .inner
            .pair_iter()
            .filter_map(|(h, n)| {
                if n.is_resource_instance_root {
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
                            .scene
                            .graph
                            .is_valid_handle(node.original_handle_in_resource)
                        {
                            nodes_to_delete.push(node.self_handle);

                            Log::warn(format!(
                                "Node {} ({}:{}) and its children will be deleted, because it \
                    does not exist in the parent asset `{}`!",
                                node.name(),
                                node.self_handle.index(),
                                node.self_handle.generation(),
                                kind
                            ))
                        }
                    } else {
                        Log::warn(format!(
                            "Node {} ({}:{}) and its children will be deleted, because its \
                    parent asset `{}` failed to load!",
                            node.name(),
                            node.self_handle.index(),
                            node.self_handle.generation(),
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
                let resource_graph = &data.get_scene().graph;

                let resource_instance_root = self.inner[instance_root].original_handle_in_resource;

                if resource_instance_root.is_none() {
                    let instance = &self.inner[instance_root];
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
                    let resource_node = &resource_graph[resource_node_handle];

                    // Root of the resource is not belongs to resource, it is just a convenient way of
                    // consolidation all descendants under a single node.
                    let mut compare =
                        |n: &Node| n.original_handle_in_resource == resource_node_handle;

                    if resource_node_handle != resource_graph.inner.root
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
                        let (copy, old_to_new_mapping) = ModelResource::instantiate_from(
                            resource.clone(),
                            data,
                            resource_node_handle,
                            self,
                            &mut |_, _| {},
                        );

                        restored_count += old_to_new_mapping.map.len();

                        // Link it with existing node.
                        if resource_node.parent().is_some() {
                            let parent = self.find(instance_root, |n| {
                                n.original_handle_in_resource == resource_node.parent()
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

    fn restore_dynamic_node_data(&mut self) {
        for (handle, node) in self.inner.pair_iter_mut() {
            node.self_handle = handle;
            node.script_message_sender = Some(self.script_message_sender.clone());
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

    pub(crate) fn resolve(&mut self, resource_manager: &ResourceManager) {
        Log::writeln(MessageKind::Information, "Resolving graph...");

        self.restore_dynamic_node_data();
        self.mark_ancestor_nodes_as_modified();
        self.restore_original_handles_and_inherit_properties();
        self.update_hierarchical_data();
        let instances = self.restore_integrity();
        self.remap_handles(&instances);

        // Update cube maps for sky boxes.
        for node in self.linear_iter_mut() {
            if let Some(camera) = node.cast_mut::<Camera>() {
                if let Some(skybox) = camera.skybox_mut() {
                    Log::verify(skybox.create_cubemap());
                }
            }
        }

        // Sync materials with shaders.
        let mut materials = FxHashSet::default();
        for node in self.linear_iter_mut() {
            (node as &mut dyn Reflect).enumerate_fields_recursively(
                &mut |_, _, v| {
                    v.downcast_ref::<MaterialResource>(&mut |material| {
                        if let Some(material) = material {
                            materials.insert(material.clone());
                        }
                    })
                },
                &[TypeId::of::<UntypedResource>()],
            );
        }

        for material in materials {
            let mut material_state = material.state();
            if let Some(material) = material_state.data() {
                material.sync_to_shader(resource_manager);
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
                        if let Err(e) = material.set_property(
                            &ImmutableString::new("lightmapTexture"),
                            PropertyValue::Sampler {
                                value: Some(texture),
                                fallback: SamplerFallback::Black,
                            },
                        ) {
                            Log::writeln(
                                MessageKind::Error,
                                format!(
                                    "Failed to apply light map texture to material. Reason {:?}",
                                    e
                                ),
                            )
                        }
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
                if let Some(mesh) = self.inner[handle].cast_mut::<Mesh>() {
                    for surface in mesh.surfaces() {
                        let data = surface.data();
                        unique_data_set.entry(data.key()).or_insert(data);
                    }
                }
            }

            for (_, data) in unique_data_set.into_iter() {
                let mut data = data.lock();

                if let Some(patch) = lightmap.patches.get(&data.content_hash()) {
                    if !data
                        .vertex_buffer
                        .has_attribute(VertexAttributeUsage::TexCoord1)
                    {
                        data.vertex_buffer
                            .modify()
                            .add_attribute(
                                VertexAttributeDescriptor {
                                    usage: VertexAttributeUsage::TexCoord1,
                                    data_type: VertexAttributeDataType::F32,
                                    size: 2,
                                    divisor: 0,
                                    shader_location: 6, // HACK: GBuffer renderer expects it to be at 6
                                    normalized: false,
                                },
                                Vector2::<f32>::default(),
                            )
                            .unwrap();
                    }

                    data.geometry_buffer.set_triangles(patch.triangles.clone());

                    let mut vertex_buffer_mut = data.vertex_buffer.modify();
                    for &v in patch.additional_vertices.iter() {
                        vertex_buffer_mut.duplicate(v as usize);
                    }

                    assert_eq!(
                        vertex_buffer_mut.vertex_count() as usize,
                        patch.second_tex_coords.len()
                    );
                    for (mut view, &tex_coord) in vertex_buffer_mut
                        .iter_mut()
                        .zip(patch.second_tex_coords.iter())
                    {
                        view.write_2_f32(VertexAttributeUsage::TexCoord1, tex_coord)
                            .unwrap();
                    }
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
                if let Some(mesh) = self.inner[handle].cast_mut::<Mesh>() {
                    for (entry, surface) in entries.iter_mut().zip(mesh.surfaces_mut()) {
                        let mut material_state = surface.material().state();
                        if let Some(material) = material_state.data() {
                            if let Err(e) = material.set_property(
                                &ImmutableString::new("lightmapTexture"),
                                PropertyValue::Sampler {
                                    value: entry.texture.clone(),
                                    fallback: SamplerFallback::Black,
                                },
                            ) {
                                Log::writeln(
                                    MessageKind::Error,
                                    format!(
                                    "Failed to apply light map texture to material. Reason {:?}",
                                    e
                                ),
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn update_hierarchical_data_recursively(
        nodes: &LowLevelGraph,
        sound_context: &mut SoundContext,
        physics: &mut PhysicsWorld,
        physics2d: &mut dim2::physics::PhysicsWorld,
        node_handle: Handle<Node>,
    ) {
        let node = &nodes[node_handle];

        let (parent_global_transform, parent_visibility, parent_enabled) =
            if let Some(parent) = nodes.try_borrow(node.parent()) {
                (
                    parent.global_transform(),
                    parent.global_visibility(),
                    parent.is_globally_enabled(),
                )
            } else {
                (Matrix4::identity(), true, true)
            };

        let new_global_transform = parent_global_transform * node.local_transform().matrix();

        // TODO: Detect changes from user code here.
        node.sync_transform(
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
        node.global_visibility
            .set(parent_visibility && node.visibility());
        node.global_enabled.set(parent_enabled && node.is_enabled());

        for &child in node.children() {
            Self::update_hierarchical_data_recursively(
                nodes,
                sound_context,
                physics,
                physics2d,
                child,
            );
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

    /// Calculates local and global transform, global visibility for each node in graph starting from the
    /// specified node and down the tree. The main use case of the method is to update global position (etc.)
    /// of an hierarchy of the nodes of some new prefab instance.
    #[inline]
    pub fn update_hierarchical_data_for_descendants(&mut self, node_handle: Handle<Node>) {
        Self::update_hierarchical_data_recursively(
            &self.inner,
            &mut self.sound_context,
            &mut self.physics,
            &mut self.physics2d,
            node_handle,
        );
    }

    /// Calculates local and global transform, global visibility for each node in graph.
    /// Normally you not need to call this method directly, it will be called automatically
    /// on each frame. However there is one use case - when you setup complex hierarchy and
    /// need to know global transform of nodes before entering update loop, then you can call
    /// this method.
    #[inline]
    pub fn update_hierarchical_data(&mut self) {
        Self::update_hierarchical_data_recursively(
            &self.inner,
            &mut self.sound_context,
            &mut self.physics,
            &mut self.physics2d,
            self.inner.root,
        );
    }

    /// Checks whether given node handle is valid or not.
    #[inline]
    pub fn is_valid_handle(&self, node_handle: Handle<Node>) -> bool {
        self.inner.is_valid_handle(node_handle)
    }

    fn sync_native(&mut self, switches: &GraphUpdateSwitches) {
        let mut sync_context = SyncContext {
            nodes: &self.inner,
            physics: &mut self.physics,
            physics2d: &mut self.physics2d,
            sound_context: &mut self.sound_context,
            switches: Some(switches),
        };

        for (handle, node) in self.inner.pair_iter() {
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
        if let Some((ticket, mut node)) = self.inner.try_take_reserve(handle) {
            node.transform_modified.set(false);

            let mut is_alive = node.is_alive();

            if node.is_globally_enabled() {
                node.update(&mut UpdateContext {
                    frame_size,
                    dt,
                    nodes: &mut self.inner,
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

            self.inner.put_back(ticket, node);

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
        self.update_hierarchical_data();
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
            for i in 0..self.inner.get_capacity() {
                self.update_node(
                    self.inner.handle_from_index(i),
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
    /// use fyrox::scene::node::Node;
    /// use fyrox::scene::graph::Graph;
    /// use fyrox::scene::pivot::Pivot;
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
        self.inner.get_capacity()
    }

    /// Makes new handle from given index. Handle will be none if index was either out-of-bounds
    /// or point to a vacant pool entry.
    ///
    /// ```
    /// use fyrox::scene::node::Node;
    /// use fyrox::scene::graph::Graph;
    /// use fyrox::scene::pivot::Pivot;
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
        self.inner.handle_from_index(index)
    }

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    #[inline]
    pub fn linear_iter(&self) -> impl Iterator<Item = &Node> {
        self.inner.iter()
    }

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    #[inline]
    pub fn linear_iter_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.inner.iter_mut()
    }

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    #[inline]
    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Node>, &Node)> {
        self.inner.pair_iter()
    }

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    #[inline]
    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (Handle<Node>, &mut Node)> {
        self.inner.pair_iter_mut()
    }

    /// Extracts node from graph and reserves its handle. It is used to temporarily take
    /// ownership over node, and then put node back using given ticket. Extracted node is
    /// detached from its parent!
    #[inline]
    pub fn take_reserve(&mut self, handle: Handle<Node>) -> (Ticket<Node>, Node) {
        self.unlink_internal(handle);
        let (ticket, mut node) = self.inner.take_reserve(handle);
        Self::on_node_removed(
            &self.inner.begin_multi_borrow(),
            &mut node,
            &mut self.physics,
            &mut self.physics2d,
            &mut self.sound_context,
            &mut self.instance_id_map,
        );
        self.instance_id_map.remove(&node.instance_id);
        (ticket, node)
    }

    /// Puts node back by given ticket. Attaches back to root node of graph.
    #[inline]
    pub fn put_back(&mut self, ticket: Ticket<Node>, node: Node) -> Handle<Node> {
        let instance_id = node.instance_id;
        let handle = self.inner.put_back(ticket, node);
        self.instance_id_map.insert(instance_id, handle);
        handle
    }

    /// Makes node handle vacant again.
    #[inline]
    pub fn forget_ticket(&mut self, ticket: Ticket<Node>, node: Node) -> Node {
        self.inner.forget_node_ticket(ticket, node)
    }

    /// Extracts sub-graph starting from a given node. All handles to extracted nodes
    /// becomes reserved and will be marked as "occupied", an attempt to borrow a node
    /// at such handle will result in panic!. Please note that root node will be
    /// detached from its parent!
    #[inline]
    pub fn take_reserve_sub_graph(&mut self, root: Handle<Node>) -> SubGraph {
        let mut subgraph = self.inner.take_reserve_sub_graph(root);

        for (_, node) in subgraph.descendants.iter_mut().chain([&mut subgraph.root]) {
            Self::on_node_removed(
                &self.inner.begin_multi_borrow(),
                node,
                &mut self.physics,
                &mut self.physics2d,
                &mut self.sound_context,
                &mut self.instance_id_map,
            );
        }

        subgraph.root.1.on_unlink(&mut GenericContext {
            nodes: &self.inner.begin_multi_borrow(),
            physics: &mut self.physics,
            physics2d: &mut self.physics2d,
            sound_context: &mut self.sound_context,
        });

        subgraph
    }

    /// Puts previously extracted sub-graph into graph. Handles to nodes will become valid
    /// again. After that you probably want to re-link returned handle with its previous
    /// parent.
    #[inline]
    pub fn put_sub_graph_back(&mut self, sub_graph: SubGraph) -> Handle<Node> {
        self.inner.put_sub_graph_back(sub_graph)
    }

    /// Forgets the entire sub-graph making handles to nodes invalid.
    #[inline]
    pub fn forget_sub_graph(&mut self, sub_graph: SubGraph) {
        self.inner.forget_sub_graph(sub_graph)
    }

    /// Returns the number of nodes in the graph.
    #[inline]
    pub fn node_count(&self) -> u32 {
        self.inner.alive_count()
    }

    /// Create a graph depth traversal iterator.
    ///
    /// # Notes
    ///
    /// This method allocates temporal array so it is not cheap! Should not be
    /// used on each frame.
    #[inline]
    pub fn traverse_iter(&self, from: Handle<Node>) -> GraphTraverseIterator {
        GraphTraverseIterator {
            graph: self,
            stack: vec![from],
        }
    }

    /// Create a graph depth traversal iterator which will emit *handles* to nodes.
    ///
    /// # Notes
    ///
    /// This method allocates temporal array so it is not cheap! Should not be
    /// used on each frame.
    #[inline]
    pub fn traverse_handle_iter(&self, from: Handle<Node>) -> GraphHandleTraverseIterator {
        GraphHandleTraverseIterator {
            graph: self,
            stack: vec![from],
        }
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
    ) -> (Self, NodeHandleMap)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
        Pre: FnMut(Handle<Node>, &mut Node),
        Post: FnMut(Handle<Node>, Handle<Node>, &mut Node),
    {
        let mut copy = Self {
            sound_context: self.sound_context.deep_clone(),
            ..Default::default()
        };

        let (copy_root, old_new_map) = self.copy_node(
            root,
            &mut copy,
            filter,
            pre_process_callback,
            post_process_callback,
        );
        assert_eq!(copy.inner.root, copy_root);

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
        isometric_local_transform(&self.inner, node)
    }

    /// Returns world transformation matrix of a node only.  Such transform has
    /// only translation and rotation.
    #[inline]
    pub fn isometric_global_transform(&self, node: Handle<Node>) -> Matrix4<f32> {
        isometric_global_transform(&self.inner, node)
    }

    /// Returns global scale matrix of a node.
    #[inline]
    pub fn global_scale_matrix(&self, node: Handle<Node>) -> Matrix4<f32> {
        let node = &self[node];
        let local_scale_matrix = Matrix4::new_nonuniform_scaling(node.local_transform().scale());
        if node.parent().is_some() {
            self.global_scale_matrix(node.parent()) * local_scale_matrix
        } else {
            local_scale_matrix
        }
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
    pub fn global_scale(&self, node: Handle<Node>) -> Vector3<f32> {
        let m = self.global_scale_matrix(node);
        Vector3::new(m[0], m[5], m[10])
    }

    /// Tries to borrow a node using the given handle, fetch its script and cast it to the specified type.
    #[inline]
    pub fn try_get_script_of<T>(&self, node: Handle<Node>) -> Option<&T>
    where
        T: ScriptTrait,
    {
        self.try_get(node).and_then(|node| node.try_get_script())
    }

    /// Tries to borrow a node using the given handle, fetch its script and cast it to the specified type.
    #[inline]
    pub fn try_get_script_of_mut<T>(&mut self, node: Handle<Node>) -> Option<&mut T>
    where
        T: ScriptTrait,
    {
        self.try_get_mut(node)
            .and_then(|node| node.try_get_script_mut())
    }

    /// Tries to borrow a node using the given handle and fetch a reference to a component of the given type
    /// from the script of the node.
    #[inline]
    pub fn try_get_script_component_of<C>(&self, node: Handle<Node>) -> Option<&C>
    where
        C: Any,
    {
        self.try_get(node)
            .and_then(|node| node.try_get_script_component())
    }

    /// Tries to borrow a node using the given handle and fetch a reference to a component of the given type
    /// from the script of the node.
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
            .and_then(|h| self.inner.try_borrow(*h).map(|n| (*h, n)))
    }

    /// Tries to borrow a node by its id.
    pub fn node_by_id_mut(&mut self, id: SceneNodeId) -> Option<(Handle<Node>, &mut Node)> {
        self.instance_id_map
            .get(&id)
            .and_then(|h| self.inner.try_borrow_mut(*h).map(|n| (*h, n)))
    }
}

impl Index<Handle<Node>> for Graph {
    type Output = Node;

    #[inline]
    fn index(&self, index: Handle<Node>) -> &Self::Output {
        &self.inner[index]
    }
}

impl IndexMut<Handle<Node>> for Graph {
    #[inline]
    fn index_mut(&mut self, index: Handle<Node>) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T> Index<Handle<T>> for Graph
where
    T: NodeTrait,
{
    type Output = T;

    #[inline]
    fn index(&self, typed_handle: Handle<T>) -> &Self::Output {
        let node = &self.inner[typed_handle.transmute()];
        node.cast().unwrap_or_else(|| {
            panic!(
                "Downcasting of node {} ({}:{}) to type {} failed!",
                node.name(),
                typed_handle.index(),
                typed_handle.generation(),
                node.type_name()
            )
        })
    }
}

impl<T> IndexMut<Handle<T>> for Graph
where
    T: NodeTrait,
{
    #[inline]
    fn index_mut(&mut self, typed_handle: Handle<T>) -> &mut Self::Output {
        let node = &mut self.inner[typed_handle.transmute()];

        // SAFETY: This is safe to do, because we only read node's values for panicking.
        let second_node_ref = unsafe { &*(node as *const Node) };

        if let Some(downcasted) = node.cast_mut() {
            downcasted
        } else {
            panic!(
                "Downcasting of node {} ({}:{}) to type {} failed!",
                second_node_ref.name(),
                typed_handle.index(),
                typed_handle.generation(),
                second_node_ref.type_name()
            )
        }
    }
}

/// Iterator that traverses tree in depth and returns shared references to nodes.
pub struct GraphTraverseIterator<'a> {
    graph: &'a Graph,
    stack: Vec<Handle<Node>>,
}

impl<'a> Iterator for GraphTraverseIterator<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(handle) = self.stack.pop() {
            let node = &self.graph[handle];

            for child_handle in node.children() {
                self.stack.push(*child_handle);
            }

            return Some(node);
        }

        None
    }
}

/// Iterator that traverses tree in depth and returns handles to nodes.
pub struct GraphHandleTraverseIterator<'a> {
    graph: &'a Graph,
    stack: Vec<Handle<Node>>,
}

impl<'a> Iterator for GraphHandleTraverseIterator<'a> {
    type Item = Handle<Node>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(handle) = self.stack.pop() {
            for child_handle in self.graph[handle].children() {
                self.stack.push(*child_handle);
            }

            return Some(handle);
        }
        None
    }
}

impl Visit for Graph {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        // Pool must be empty, otherwise handles will be invalid and everything will blow up.
        if visitor.is_reading() && self.inner.get_capacity() != 0 {
            panic!("Graph pool must be empty on load!")
        }

        let mut region = visitor.enter_region(name)?;

        self.inner.visit("Root", "Pool", &mut region)?;
        self.sound_context.visit("SoundContext", &mut region)?;
        self.physics.visit("PhysicsWorld", &mut region)?;
        self.physics2d.visit("PhysicsWorld2D", &mut region)?;
        let _ = self.lightmap.visit("Lightmap", &mut region);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        asset::{io::FsResourceIo, manager::ResourceManager},
        core::{
            algebra::{Matrix4, Vector3},
            futures::executor::block_on,
            pool::Handle,
            visitor::Visitor,
        },
        engine::{self, SerializationContext},
        resource::model::{Model, ModelResourceExtension},
        scene::{
            base::BaseBuilder,
            graph::Graph,
            mesh::{
                surface::{SurfaceBuilder, SurfaceData, SurfaceSharedData},
                MeshBuilder,
            },
            node::Node,
            pivot::{Pivot, PivotBuilder},
            transform::TransformBuilder,
            Scene, SceneLoader,
        },
    };
    use std::{fs, path::Path, sync::Arc};

    #[test]
    fn graph_init_test() {
        let graph = Graph::new();
        assert_ne!(graph.root, Handle::NONE);
        assert_eq!(graph.inner.alive_count(), 1);
    }

    #[test]
    fn graph_node_test() {
        let mut graph = Graph::new();
        graph.add_node(Node::new(Pivot::default()));
        graph.add_node(Node::new(Pivot::default()));
        graph.add_node(Node::new(Pivot::default()));
        assert_eq!(graph.inner.alive_count(), 4);
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

    #[test]
    fn test_change_root() {
        let mut graph = Graph::new();

        // Root_
        //      |_A_
        //          |_B
        //          |_C_
        //             |_D
        let root = graph.root;
        let b;
        let c;
        let d;
        let a = PivotBuilder::new(BaseBuilder::new().with_children(&[
            {
                b = PivotBuilder::new(BaseBuilder::new()).build(&mut graph);
                b
            },
            {
                c = PivotBuilder::new(BaseBuilder::new().with_children(&[{
                    d = PivotBuilder::new(BaseBuilder::new()).build(&mut graph);
                    d
                }]))
                .build(&mut graph);
                c
            },
        ]))
        .build(&mut graph);

        dbg!(root, a, b, c, d);

        graph.change_root_inplace(c);

        // C_
        //      |_D
        //      |_A_
        //          |_B
        //      |_Root
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
            .with_surfaces(vec![SurfaceBuilder::new(SurfaceSharedData::new(
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
    }

    fn make_resource_manager() -> ResourceManager {
        let resource_manager = ResourceManager::new(Arc::new(Default::default()));
        engine::initialize_resource_manager_loaders(
            &resource_manager,
            Arc::new(SerializationContext::new()),
        );
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
                .finish(&resource_manager),
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
}
