pub mod node;

use crate::utils::pool::*;
use crate::math::mat4::*;
use node::*;

pub struct Scene {
    /// Nodes pool, every node lies inside pool. User-code may borrow
    /// a reference to a node using handle.
    pub(crate) nodes: Pool<Node>,

    /// Root node of scene. Each node added to scene will be attached
    /// to root.
    pub(crate) root: Handle<Node>,

    /// Tree traversal stack
    stack: Vec<Handle<Node>>,
}

impl Scene {
    pub fn new() -> Scene {
        let mut nodes: Pool<Node> = Pool::new();
        let root = nodes.spawn(Node::new(NodeKind::Base));
        Scene {
            nodes,
            stack: Vec::new(),
            root,
        }
    }

    /// Transfers ownership of node into scene.
    /// Returns handle to node.
    pub fn add_node(&mut self, node: Node) -> Handle<Node> {
        let handle = self.nodes.spawn(node);
        self.link_nodes(&handle, &self.root.clone());
        handle
    }

    /// Destroys node
    pub fn remove_node(&mut self, handle: Handle<Node>) {
        self.nodes.free(handle);
    }

    pub fn borrow_node(&self, handle: &Handle<Node>) -> Option<&Node> {
        self.nodes.borrow(handle)
    }

    pub fn borrow_node_mut(&mut self, handle: &Handle<Node>) -> Option<&mut Node> {
        self.nodes.borrow_mut(handle)
    }

    /// Links specified child with specified parent.
    pub fn link_nodes(&mut self, child_handle: &Handle<Node>, parent_handle: &Handle<Node>) {
        self.unlink_node(child_handle);
        if let Some(child) = self.nodes.borrow_mut(child_handle) {
            child.parent = parent_handle.clone();
            if let Some(parent) = self.nodes.borrow_mut(parent_handle) {
                parent.children.push(child_handle.clone());
            }
        }
    }

    pub fn unlink_node(&mut self, node_handle: &Handle<Node>) {
        let mut parent_handle: Handle<Node> = Handle::none();
        // Replace parent handle of child
        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            parent_handle = node.parent.clone();
            node.parent = Handle::none();
        }
        // Remove child from parent's children list
        if let Some(parent) = self.nodes.borrow_mut(&parent_handle) {
            if let Some(i) = parent.children.iter().position(|h| h == node_handle) {
                parent.children.remove(i);
            }
        }
    }

    pub fn update(&mut self, aspect_ratio: f32) {
        // Calculate transforms on nodes
        self.stack.clear();
        self.stack.push(self.root.clone());
        loop {
            match self.stack.pop() {
                Some(handle) => {
                    // Calculate local transform and get parent handle
                    let mut parent_handle: Handle<Node> = Handle::none();
                    if let Some(node) = self.nodes.borrow_mut(&handle) {
                        node.calculate_local_transform();
                        parent_handle = node.parent.clone();
                    }

                    // Extract parent's global transform
                    let mut parent_global_transform = Mat4::identity();
                    if let Some(parent) = self.nodes.borrow_mut(&parent_handle) {
                        parent_global_transform = parent.global_transform;
                    }

                    if let Some(node) = self.nodes.borrow_mut(&handle) {
                        node.global_transform = parent_global_transform * node.local_transform;
                        let eye = node.get_global_position();
                        let look = node.get_look_vector();
                        let up = node.get_up_vector();

                        if let NodeKind::Camera(camera) = node.borrow_kind_mut() {
                            camera.calculate_matrices(eye, look, up, aspect_ratio);
                        }

                        // Queue children and continue traversal on them
                        for child_handle in node.children.iter() {
                            self.stack.push(child_handle.clone());
                        }
                    }
                }
                None => break
            }
        }
    }
}