use crate::{
    core::{
        algebra::{Matrix4, Rotation2, UnitComplex, Vector2, Vector3},
        pool::{Handle, Pool, Ticket},
        visitor::prelude::*,
    },
    scene2d::{node::Node, transform::TransformBuilder},
};
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

#[derive(Default, Visit)]
pub struct Graph {
    pool: Pool<Node>,
    root: Handle<Node>,
    #[visit(skip)]
    stack: Vec<Handle<Node>>,
}

impl Graph {
    /// Creates new graph instance with single root node.
    pub fn new() -> Self {
        let mut pool = Pool::new();
        let mut root = Node::Base(Default::default());
        root.set_name("__ROOT__");
        let root = pool.spawn(root);
        Self {
            stack: Vec::new(),
            root,
            pool,
        }
    }

    /// Adds new node to the graph. Node will be transferred into implementation-defined
    /// storage and you'll get a handle to the node. Node will be automatically attached
    /// to root node of graph, it is required because graph can contain only one root.
    #[inline]
    pub fn add_node(&mut self, mut node: Node) -> Handle<Node> {
        let children = node.children.clone();
        node.children.clear();
        let handle = self.pool.spawn(node);
        if self.root.is_some() {
            self.link_nodes(handle, self.root);
        }
        for child in children {
            self.link_nodes(child, handle);
        }
        handle
    }

    /// Tries to borrow mutable references to two nodes at the same time by given handles. Will
    /// panic if handles overlaps (points to same node).
    pub fn get_two_mut(&mut self, nodes: (Handle<Node>, Handle<Node>)) -> (&mut Node, &mut Node) {
        self.pool.borrow_two_mut(nodes)
    }

    /// Tries to borrow mutable references to three nodes at the same time by given handles. Will
    /// return Err of handles overlaps (points to same node).
    pub fn get_three_mut(
        &mut self,
        nodes: (Handle<Node>, Handle<Node>, Handle<Node>),
    ) -> (&mut Node, &mut Node, &mut Node) {
        self.pool.borrow_three_mut(nodes)
    }

    /// Tries to borrow mutable references to four nodes at the same time by given handles. Will
    /// panic if handles overlaps (points to same node).
    pub fn get_four_mut(
        &mut self,
        nodes: (Handle<Node>, Handle<Node>, Handle<Node>, Handle<Node>),
    ) -> (&mut Node, &mut Node, &mut Node, &mut Node) {
        self.pool.borrow_four_mut(nodes)
    }

    /// Returns root node of current graph.
    pub fn get_root(&self) -> Handle<Node> {
        self.root
    }

    /// Destroys node and its children recursively.
    ///
    /// # Notes
    ///
    /// This method does not remove references to the node in other places like animations,
    /// physics, etc. You should prefer to use [Scene::remove_node](crate::scene::Scene::remove_node) -
    /// it automatically breaks all associations between nodes.
    #[inline]
    pub fn remove_node(&mut self, node_handle: Handle<Node>) {
        self.unlink_internal(node_handle);

        self.stack.clear();
        self.stack.push(node_handle);
        while let Some(handle) = self.stack.pop() {
            for &child in self.pool[handle].children().iter() {
                self.stack.push(child);
            }
            self.pool.free(handle);
        }
    }

    fn unlink_internal(&mut self, node_handle: Handle<Node>) {
        // Replace parent handle of child
        let parent_handle = std::mem::replace(&mut self.pool[node_handle].parent, Handle::NONE);

        // Remove child from parent's children list
        if parent_handle.is_some() {
            let parent = &mut self.pool[parent_handle];
            if let Some(i) = parent.children().iter().position(|h| *h == node_handle) {
                parent.children.remove(i);
            }
        }
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child: Handle<Node>, parent: Handle<Node>) {
        self.unlink_internal(child);
        self.pool[child].parent = parent;
        self.pool[parent].children.push(child);
    }

    /// Unlinks specified node from its parent and attaches it to root graph node.
    #[inline]
    pub fn unlink_node(&mut self, node_handle: Handle<Node>) {
        self.unlink_internal(node_handle);
        self.link_nodes(node_handle, self.root);
        self.pool[node_handle]
            .local_transform_mut()
            .set_position(Vector2::default());
    }

    pub fn capacity(&self) -> usize {
        self.pool.get_capacity()
    }

    /// Makes new handle from given index. Handle will be none if index was either out-of-bounds
    /// or point to a vacant pool entry.
    pub fn handle_from_index(&self, index: usize) -> Handle<Node> {
        self.pool.handle_from_index(index)
    }

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    pub fn linear_iter(&self) -> impl Iterator<Item = &Node> {
        self.pool.iter()
    }

    /// Creates an iterator that has linear iteration order over internal collection
    /// of nodes. It does *not* perform any tree traversal!
    pub fn linear_iter_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.pool.iter_mut()
    }

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Node>, &Node)> {
        self.pool.pair_iter()
    }

    /// Creates new iterator that iterates over internal collection giving (handle; node) pairs.
    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (Handle<Node>, &mut Node)> {
        self.pool.pair_iter_mut()
    }

    /// Extracts node from graph and reserves its handle. It is used to temporarily take
    /// ownership over node, and then put node back using given ticket. Extracted node is
    /// detached from its parent!
    pub fn take_reserve(&mut self, handle: Handle<Node>) -> (Ticket<Node>, Node) {
        self.unlink_internal(handle);
        self.pool.take_reserve(handle)
    }

    /// Puts node back by given ticket. Attaches back to root node of graph.
    pub fn put_back(&mut self, ticket: Ticket<Node>, node: Node) -> Handle<Node> {
        let handle = self.pool.put_back(ticket, node);
        self.link_nodes(handle, self.root);
        handle
    }

    /// Makes node handle vacant again.
    pub fn forget_ticket(&mut self, ticket: Ticket<Node>) {
        self.pool.forget_ticket(ticket)
    }

    pub fn update(&mut self, render_target_size: Vector2<f32>, _dt: f32) {
        self.update_hierarchical_data();

        for node in self.pool.iter_mut() {
            if let Node::Camera(camera) = node {
                camera.update(render_target_size);
            }
        }
    }

    /// Checks whether given node handle is valid or not.
    pub fn is_valid_handle(&self, node_handle: Handle<Node>) -> bool {
        self.pool.is_valid_handle(node_handle)
    }

    /// Calculates local and global transform, global visibility for each node in graph.
    /// Normally you not need to call this method directly, it will be called automatically
    /// on each frame. However there is one use case - when you setup complex hierarchy and
    /// need to know global transform of nodes before entering update loop, then you can call
    /// this method.
    pub fn update_hierarchical_data(&mut self) {
        fn update_recursively(graph: &Graph, node_handle: Handle<Node>) {
            let node = &graph.pool[node_handle];

            let (parent_global_transform, parent_visibility) =
                if let Some(parent) = graph.pool.try_borrow(node.parent()) {
                    (parent.global_transform(), parent.global_visibility())
                } else {
                    (Matrix4::identity(), true)
                };

            node.global_transform
                .set(parent_global_transform * node.local_transform().matrix());
            node.global_visibility
                .set(parent_visibility && node.visibility());

            for &child in node.children() {
                update_recursively(graph, child);
            }
        }

        update_recursively(self, self.root);
    }

    /// Returns local transformation matrix of a node without scale.
    pub fn local_transform_no_scale(&self, node: Handle<Node>) -> Matrix4<f32> {
        self[node]
            .local_transform()
            .clone()
            .set_scale(Vector2::new(1.0, 1.0))
            .matrix()
    }

    /// Returns world transformation matrix of a node without scale.
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
    pub fn isometric_local_transform(&self, node: Handle<Node>) -> Matrix4<f32> {
        let transform = self[node].local_transform();
        TransformBuilder::new()
            .with_position(transform.position())
            .with_rotation(transform.rotation())
            .build()
            .matrix()
    }

    /// Returns world transformation matrix of a node only.  Such transform has
    /// only translation and rotation.
    pub fn isometric_global_transform(&self, node: Handle<Node>) -> Matrix4<f32> {
        let parent = self[node].parent();
        if parent.is_some() {
            self.isometric_global_transform(parent) * self.isometric_local_transform(node)
        } else {
            self.isometric_local_transform(node)
        }
    }

    /// Returns global scale matrix of a node.
    pub fn global_scale_matrix(&self, node: Handle<Node>) -> Matrix4<f32> {
        let node = &self[node];
        let scale = node.local_transform().scale();
        let local_scale_matrix =
            Matrix4::new_nonuniform_scaling(&Vector3::new(scale.x, scale.y, 1.0));
        if node.parent().is_some() {
            self.global_scale_matrix(node.parent()) * local_scale_matrix
        } else {
            local_scale_matrix
        }
    }

    /// Returns rotation quaternion of a node in world coordinates.
    pub fn global_rotation(&self, node: Handle<Node>) -> UnitComplex<f32> {
        UnitComplex::from(Rotation2::from_matrix(
            &self
                .global_transform_no_scale(node)
                .fixed_resize::<2, 2>(f32::default()),
        ))
    }

    /// Returns rotation quaternion of a node in world coordinates without pre- and post-rotations.
    pub fn isometric_global_rotation(&self, node: Handle<Node>) -> UnitComplex<f32> {
        UnitComplex::from(Rotation2::from_matrix(
            &self
                .isometric_global_transform(node)
                .fixed_resize::<2, 2>(f32::default()),
        ))
    }

    /// Returns rotation quaternion and position of a node in world coordinates, scale is eliminated.
    pub fn global_rotation_position_no_scale(
        &self,
        node: Handle<Node>,
    ) -> (UnitComplex<f32>, Vector2<f32>) {
        (self.global_rotation(node), self[node].global_position())
    }

    /// Returns isometric global rotation and position.
    pub fn isometric_global_rotation_position(
        &self,
        node: Handle<Node>,
    ) -> (UnitComplex<f32>, Vector2<f32>) {
        (
            self.isometric_global_rotation(node),
            self[node].global_position(),
        )
    }

    /// Returns global scale of a node.
    pub fn global_scale(&self, node: Handle<Node>) -> Vector2<f32> {
        let m = self.global_scale_matrix(node);
        Vector2::new(m[0], m[5])
    }

    /// Creates deep copy of node with all children. This is relatively heavy operation!
    /// In case if any error happened it returns `Handle::NONE`.
    ///
    /// # Implementation notes
    ///
    /// Returns tuple where first element is handle to copy of node, and second element -
    /// old-to-new hash map, which can be used to easily find copy of node by its original.
    ///
    /// Filter allows to exclude some nodes from copied hierarchy. It must return false for
    /// odd nodes. Filtering applied only to descendant nodes.
    pub fn copy_node<F>(
        &self,
        node_handle: Handle<Node>,
        dest_graph: &mut Graph,
        filter: &mut F,
    ) -> (Handle<Node>, HashMap<Handle<Node>, Handle<Node>>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let mut old_new_mapping = HashMap::new();
        let root_handle = self.copy_node_raw(node_handle, dest_graph, &mut old_new_mapping, filter);
        (root_handle, old_new_mapping)
    }

    /// Creates deep copy of node with all children. This is relatively heavy operation!
    /// In case if any error happened it returns `Handle::NONE`.
    ///
    /// # Notes
    ///
    /// This method has exactly the same functionality as `copy_node`, but copies not in-place.
    ///
    /// # Implementation notes
    ///
    /// Returns tuple where first element is handle to copy of node, and second element -
    /// old-to-new hash map, which can be used to easily find copy of node by its original.
    ///
    /// Filter allows to exclude some nodes from copied hierarchy. It must return false for
    /// odd nodes. Filtering applied only to descendant nodes.
    pub fn copy_node_inplace<F>(
        &mut self,
        node_handle: Handle<Node>,
        filter: &mut F,
    ) -> (Handle<Node>, HashMap<Handle<Node>, Handle<Node>>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let mut old_new_mapping = HashMap::new();

        let to_copy = self
            .traverse_handle_iter(node_handle)
            .map(|node| (node, self.pool[node].children.clone()))
            .collect::<Vec<_>>();

        let mut root_handle = Handle::NONE;

        for (parent, children) in to_copy.iter() {
            // Copy parent first.
            let parent_copy = self.pool[*parent].raw_copy();
            let parent_copy_handle = self.add_node(parent_copy);
            old_new_mapping.insert(*parent, parent_copy_handle);

            if root_handle.is_none() {
                root_handle = parent_copy_handle;
            }

            // Copy children and link to new parent.
            for &child in children {
                if filter(child, &self.pool[child]) {
                    let child_copy = self.pool[child].raw_copy();
                    let child_copy_handle = self.add_node(child_copy);
                    old_new_mapping.insert(child, child_copy_handle);
                    self.link_nodes(child_copy_handle, parent_copy_handle);
                }
            }
        }

        (root_handle, old_new_mapping)
    }

    /// Creates copy of a node and breaks all connections with other nodes.
    pub fn copy_single_node(&self, node_handle: Handle<Node>) -> Node {
        let node = &self.pool[node_handle];
        let mut clone = node.raw_copy();
        clone.parent = Handle::NONE;
        clone.children.clear();
        clone
    }

    fn copy_node_raw<F>(
        &self,
        root_handle: Handle<Node>,
        dest_graph: &mut Graph,
        old_new_mapping: &mut HashMap<Handle<Node>, Handle<Node>>,
        filter: &mut F,
    ) -> Handle<Node>
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let src_node = &self.pool[root_handle];
        let dest_node = src_node.raw_copy();
        let dest_copy_handle = dest_graph.add_node(dest_node);
        old_new_mapping.insert(root_handle, dest_copy_handle);
        for &src_child_handle in src_node.children() {
            if filter(src_child_handle, &self.pool[src_child_handle]) {
                let dest_child_handle =
                    self.copy_node_raw(src_child_handle, dest_graph, old_new_mapping, filter);
                if !dest_child_handle.is_none() {
                    dest_graph.link_nodes(dest_child_handle, dest_copy_handle);
                }
            }
        }
        dest_copy_handle
    }

    /// Creates deep copy of graph. Allows filtering while copying, returns copy and
    /// old-to-new node mapping.
    pub fn clone<F>(&self, filter: &mut F) -> (Self, HashMap<Handle<Node>, Handle<Node>>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let mut copy = Self::default();
        let (root, old_new_map) = self.copy_node(self.root, &mut copy, filter);
        copy.root = root;
        (copy, old_new_map)
    }

    /// Create graph depth traversal iterator.
    ///
    /// # Notes
    ///
    /// This method allocates temporal array so it is not cheap! Should not be
    /// used on each frame.
    pub fn traverse_iter(&self, from: Handle<Node>) -> impl Iterator<Item = &Node> {
        GraphTraverseIterator {
            graph: self,
            stack: vec![from],
        }
    }

    /// Create graph depth traversal iterator which will emit *handles* to nodes.
    ///
    /// # Notes
    ///
    /// This method allocates temporal array so it is not cheap! Should not be
    /// used on each frame.
    pub fn traverse_handle_iter(
        &self,
        from: Handle<Node>,
    ) -> impl Iterator<Item = Handle<Node>> + '_ {
        GraphHandleTraverseIterator {
            graph: self,
            stack: vec![from],
        }
    }
}

impl Index<Handle<Node>> for Graph {
    type Output = Node;

    fn index(&self, index: Handle<Node>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Node>> for Graph {
    fn index_mut(&mut self, index: Handle<Node>) -> &mut Self::Output {
        &mut self.pool[index]
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
