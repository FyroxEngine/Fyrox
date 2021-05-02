use crate::{
    core::{
        algebra::Vector2,
        pool::{Handle, Pool},
    },
    scene2d::node::Node,
};

#[derive(Default)]
pub struct Graph {
    pool: Pool<Node>,
    root: Handle<Node>,
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
}
