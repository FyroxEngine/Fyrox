pub mod node;
pub mod animation;

use crate::utils::pool::*;
use crate::math::mat4::*;
use node::*;
use crate::physics::Physics;
use std::cell::RefCell;

use serde::{Serialize, Deserialize};
use crate::renderer::surface::{SurfaceSharedData, Surface};
use crate::utils::rcpool::RcPool;

#[derive(Serialize, Deserialize)]
pub struct Scene {
    /// Nodes pool, every node lies inside pool. User-code may borrow
    /// a reference to a node using handle.
    pub(crate) nodes: Pool<Node>,

    /// Root node of scene. Each node added to scene will be attached
    /// to root.
    pub(crate) root: Handle<Node>,

    /// Tree traversal stack.
    #[serde(skip)]
    stack: Vec<Handle<Node>>,

    surf_data_storage: RcPool<SurfaceSharedData>,

    physics: Physics,
}

impl Scene {
    #[inline]
    pub fn new() -> Scene {
        let mut nodes: Pool<Node> = Pool::new();
        let root = nodes.spawn(Node::new(NodeKind::Base));
        Scene {
            nodes,
            stack: Vec::new(),
            root,
            surf_data_storage: RcPool::new(),
            physics: Physics::new(),
        }
    }

    /// Transfers ownership of node into scene.
    /// Returns handle to node.
    #[inline]
    pub fn add_node(&mut self, node: Node) -> Handle<Node> {
        let handle = self.nodes.spawn(node);
        self.link_nodes(&handle, &self.root.clone());
        handle
    }

    /// Destroys node
    #[inline]
    pub fn remove_node(&mut self, handle: Handle<Node>) {
        if let Some(node) = self.nodes.borrow_mut(&handle) {
            self.physics.remove_body(node.get_body());

            if let NodeKind::Mesh(mesh) = node.borrow_kind_mut() {
                for surf in mesh.get_surfaces() {
                    self.surf_data_storage.release(surf.get_data_handle());
                }
            }
        }
        self.nodes.free(handle);
    }

    #[inline]
    pub fn borrow_node(&self, handle: &Handle<Node>) -> Option<&Node> {
        self.nodes.borrow(handle)
    }

    #[inline]
    pub fn borrow_node_mut(&mut self, handle: &Handle<Node>) -> Option<&mut Node> {
        self.nodes.borrow_mut(handle)
    }

    #[inline]
    pub fn get_physics(&self) -> &Physics {
        &self.physics
    }

    #[inline]
    pub fn get_physics_mut(&mut self) -> &mut Physics {
        &mut self.physics
    }

    #[inline]
    pub fn get_surface_data_storage(&self) -> &RcPool<SurfaceSharedData> {
        &self.surf_data_storage
    }

    #[inline]
    pub fn get_surface_data_storage_mut(&mut self) -> &mut RcPool<SurfaceSharedData> {
        &mut self.surf_data_storage
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child_handle: &Handle<Node>, parent_handle: &Handle<Node>) {
        self.unlink_node(child_handle);
        if let Some(child) = self.nodes.borrow_mut(child_handle) {
            child.parent = parent_handle.clone();
            if let Some(parent) = self.nodes.borrow_mut(parent_handle) {
                parent.children.push(child_handle.clone());
            }
        }
    }

    /// Unlinks specified node from its parent, so node will become root.
    #[inline]
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

    /// Searches node with specified name starting from specified root node.
    pub fn find_node_by_name(&self, root_node: &Handle<Node>, name: &str) -> Option<Handle<Node>> {
        match self.nodes.borrow(root_node) {
            Some(node) => {
                if node.get_name() == name {
                    Some(root_node.clone())
                } else {
                    let mut result: Option<Handle<Node>> = None;
                    for child in &node.children {
                        if let Some(child_handle) = self.find_node_by_name(child, name) {
                            result = Some(child_handle);
                            break;
                        }
                    }
                    result
                }
            }
            None => None
        }
    }

    pub fn update_physics(&mut self, dt: f64) {
        self.physics.step(dt as f32);

        // Sync node positions with assigned physics bodies
        for node in self.nodes.iter_mut() {
            if let Some(body) = self.physics.borrow_body(&node.get_body()) {
                node.set_local_position(body.get_position());
            }
        }
    }

    pub fn calculate_transforms(&mut self) {
        // Calculate transforms on nodes
        self.stack.clear();
        self.stack.push(self.root.clone());
        while let Some(handle) = self.stack.pop() {
            // Calculate local transform and get parent handle
            let mut parent_handle: Handle<Node> = Handle::none();
            if let Some(node) = self.nodes.borrow_mut(&handle) {
                node.calculate_local_transform();
                parent_handle = node.parent.clone();
            }

            // Extract parent's global transform
            let parent_global_transform =
                match self.nodes.borrow_mut(&parent_handle) {
                    Some(parent) => parent.global_transform,
                    None => Mat4::identity()
                };

            if let Some(node) = self.nodes.borrow_mut(&handle) {
                node.global_transform = parent_global_transform * node.local_transform;

                // Queue children and continue traversal on them
                for child_handle in node.children.iter() {
                    self.stack.push(child_handle.clone());
                }
            }
        }
    }

    pub fn update(&mut self, aspect_ratio: f32, dt: f64) {
        self.update_physics(dt);

        self.calculate_transforms();

        for data in self.surf_data_storage.iter_mut() {
            data.upload();
        }

        for node in self.nodes.iter_mut() {
            let eye = node.get_global_position();
            let look = node.get_look_vector();
            let up = node.get_up_vector();
            if let NodeKind::Camera(camera) = node.borrow_kind_mut() {
                camera.calculate_matrices(eye, look, up, aspect_ratio);
            }
        }
    }
}

