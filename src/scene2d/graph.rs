use crate::{
    core::{
        algebra::{Matrix4, Vector2},
        pool::{Handle, Pool, Ticket},
        visitor::prelude::*,
    },
    scene2d::node::Node,
};
use std::ops::{Index, IndexMut};

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
