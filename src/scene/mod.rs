pub mod node;
pub mod animation;
pub mod mesh;
pub mod camera;
pub mod light;
pub mod particle_system;
pub mod transform;

use crate::{
    utils::UnsafeCollectionView,
    scene::{animation::Animation, node::Node, node::NodeKind},
};
use std::{collections::HashMap};
use rg3d_physics::Physics;
use rg3d_core::{
    visitor::{Visit, VisitResult, Visitor},
    pool::{Handle, Pool},
    math::{mat4::Mat4, vec3::Vec3},
};

pub struct Scene {
    /// Nodes pool, every node lies inside pool. User-code may borrow
    /// a reference to a node using handle.
    nodes: Pool<Node>,
    /// Root node of scene. Each node added to scene will be attached
    /// to root.
    root: Handle<Node>,
    animations: Pool<Animation>,
    physics: Physics,
    /// Tree traversal stack.
    stack: Vec<Handle<Node>>,
    active_camera: Handle<Node>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            nodes: Pool::new(),
            root: Default::default(),
            physics: Physics::new(),
            stack: Vec::new(),
            animations: Pool::new(),
            active_camera: Handle::none(),
        }
    }
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
            animations: Pool::new(),
            physics: Physics::new(),
            active_camera: Handle::none(),
        }
    }

    /// Transfers ownership of node into scene.
    /// Returns handle to node.
    #[inline]
    pub fn add_node(&mut self, node: Node) -> Handle<Node> {
        let handle = self.nodes.spawn(node);
        self.link_nodes(handle, self.root);
        if let Some(node) = self.nodes.borrow(handle) {
            if let NodeKind::Camera(_) = node.get_kind() {
                self.active_camera = handle;
            }
        }
        handle
    }

    #[inline]
    pub fn get_nodes(&self) -> &Pool<Node> {
        &self.nodes
    }

    #[inline]
    pub fn get_nodes_mut(&mut self) -> &mut Pool<Node> {
        &mut self.nodes
    }

    pub fn get_active_camera(&self) -> Option<&Node> {
        self.nodes.borrow(self.active_camera)
    }

    /// Destroys node and its children recursively.
    #[inline]
    pub fn remove_node(&mut self, node_handle: Handle<Node>) {
        let mut children = UnsafeCollectionView::empty();

        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            self.physics.remove_body(node.get_rigid_body());
            children = UnsafeCollectionView::from_slice(&node.children);
        }

        // Free children recursively
        for child_handle in children.iter() {
            self.remove_node(child_handle.clone());
        }

        self.nodes.free(node_handle);
    }

    #[inline]
    pub fn get_node(&self, handle: Handle<Node>) -> Option<&Node> {
        self.nodes.borrow(handle)
    }

    #[inline]
    pub fn get_node_mut(&mut self, handle: Handle<Node>) -> Option<&mut Node> {
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
    pub fn add_animation(&mut self, animation: Animation) -> Handle<Animation> {
        self.animations.spawn(animation)
    }

    #[inline]
    pub fn get_animation(&self, handle: Handle<Animation>) -> Option<&Animation> {
        self.animations.borrow(handle)
    }

    /// Tries to find a copy of `node_handle` in hierarchy tree starting from `root_handle`.
    pub fn find_copy_of(&self, root_handle: Handle<Node>, node_handle: Handle<Node>) -> Handle<Node> {
        if let Some(root) = self.nodes.borrow(root_handle) {
            if root.get_original_handle() == node_handle {
                return root_handle;
            }

            for child_handle in root.children.iter() {
                let out = self.find_copy_of(*child_handle, node_handle);
                if out.is_some() {
                    return out;
                }
            }
        }
        Handle::none()
    }

    #[inline]
    pub fn get_animation_mut(&mut self, handle: Handle<Animation>) -> Option<&mut Animation> {
        self.animations.borrow_mut(handle)
    }

    #[inline]
    pub fn get_animations(&self) -> &Pool<Animation> {
        &self.animations
    }

    /// Links specified child with specified parent.
    #[inline]
    pub fn link_nodes(&mut self, child_handle: Handle<Node>, parent_handle: Handle<Node>) {
        self.unlink_node(child_handle);
        if let Some(child) = self.nodes.borrow_mut(child_handle) {
            child.parent = parent_handle;
            if let Some(parent) = self.nodes.borrow_mut(parent_handle) {
                parent.children.push(child_handle);
            }
        }
    }

    /// Unlinks specified node from its parent, so node will become root.
    #[inline]
    pub fn unlink_node(&mut self, node_handle: Handle<Node>) {
        let mut parent_handle: Handle<Node> = Handle::none();
        // Replace parent handle of child
        if let Some(node) = self.nodes.borrow_mut(node_handle) {
            parent_handle = node.parent;
            node.parent = Handle::none();
        }
        // Remove child from parent's children list
        if let Some(parent) = self.nodes.borrow_mut(parent_handle) {
            if let Some(i) = parent.children.iter().position(|h| *h == node_handle) {
                parent.children.remove(i);
            }
        }
    }

    /// Searches node with specified name starting from specified root node.
    pub fn find_node_by_name(&self, root_node: Handle<Node>, name: &str) -> Handle<Node> {
        match self.nodes.borrow(root_node) {
            Some(node) => {
                if node.get_name() == name {
                    root_node
                } else {
                    let mut result: Handle<Node> = Handle::none();
                    for child in &node.children {
                        let child_handle = self.find_node_by_name(*child, name);
                        if !child_handle.is_none() {
                            result = child_handle;
                            break;
                        }
                    }
                    result
                }
            }
            None => Handle::none()
        }
    }

    /// Searches node with specified name starting from specified root node.
    pub fn find_node_by_name_from_root(&self, name: &str) -> Handle<Node> {
        self.find_node_by_name(self.root, name)
    }

    /// Creates a full copy of node with all children. This is relatively heavy operation!
    /// In case if any error happened it returns [`Handle::none()`]. Automatically
    /// remaps bones for copied surfaces.
    pub fn copy_node(&self, node_handle: Handle<Node>, dest_scene: &mut Scene) -> Handle<Node> {
        let mut old_new_mapping: HashMap<Handle<Node>, Handle<Node>> = HashMap::new();
        let root_handle = self.copy_node_internal(node_handle, dest_scene, &mut old_new_mapping);

        // Iterate over instantiated nodes and remap bones handles.
        for (_, new_node_handle) in old_new_mapping.iter() {
            if let Some(node) = dest_scene.get_node_mut(*new_node_handle) {
                if let NodeKind::Mesh(mesh) = node.get_kind_mut() {
                    for surface in mesh.get_surfaces_mut() {
                        for bone_handle in surface.bones.iter_mut() {
                            if let Some(entry) = old_new_mapping.get(bone_handle) {
                                *bone_handle = *entry;
                            }
                        }
                    }
                }
            }
        }

        root_handle
    }

    fn copy_node_internal(&self, root_handle: Handle<Node>, dest_scene: &mut Scene, old_new_mapping: &mut HashMap<Handle<Node>, Handle<Node>>) -> Handle<Node> {
        match self.get_node(root_handle) {
            Some(src_node) => {
                let mut dest_node = src_node.make_copy(root_handle);
                if let Some(src_body) = self.physics.borrow_body(src_node.get_rigid_body()) {
                    dest_node.set_rigid_body(dest_scene.physics.add_body(src_body.make_copy()));
                }
                let dest_copy_handle = dest_scene.add_node(dest_node);
                old_new_mapping.insert(root_handle, dest_copy_handle);
                for src_child_handle in &src_node.children {
                    let dest_child_handle = self.copy_node_internal(*src_child_handle, dest_scene, old_new_mapping);
                    if !dest_child_handle.is_none() {
                        dest_scene.link_nodes(dest_child_handle, dest_copy_handle);
                    }
                }
                dest_copy_handle
            }
            None => Handle::none()
        }
    }

    #[inline]
    pub fn get_root(&self) -> Handle<Node> {
        self.root
    }

    pub fn update_physics(&mut self, dt: f32) {
        self.physics.step(dt);

        // Sync node positions with assigned physics bodies
        for node in self.nodes.iter_mut() {
            if let Some(body) = self.physics.borrow_body(node.get_rigid_body()) {
                node.get_local_transform_mut().set_position(body.get_position());
            }
        }
    }

    fn find_model_root(&self, from: Handle<Node>) -> Handle<Node> {
        let mut model_root_handle = from;
        while let Some(model_node) = self.get_nodes().borrow(model_root_handle) {
            if self.get_nodes().borrow(model_node.get_parent()).is_none() {
                // We have no parent on node, then it must be root.
                return model_root_handle;
            }

            if model_node.is_resource_instance {
                return model_root_handle;
            }

            // Continue searching up on hierarchy.
            model_root_handle = model_node.get_parent();
        }
        model_root_handle
    }

    pub fn resolve(&mut self) {
        self.update_nodes();

        // Resolve original handles. Original handle is a handle to a node in resource from which
        // a node was instantiated from. We can resolve it only by names of nodes, but this is not
        // reliable way of doing this, because some editors allow nodes to have same names for
        // objects, but here we'll assume that modellers will not create models with duplicated
        // names.
        for node in self.get_nodes_mut().iter_mut() {
            if let Some(model) = node.get_resource() {
                let model = model.lock().unwrap();
                let ref_nodes = model.get_scene().get_nodes();
                for i in 0..ref_nodes.get_capacity() {
                    if let Some(resource_node) = ref_nodes.at(i) {
                        if resource_node.get_name() == node.get_name() {
                            node.set_original_handle(ref_nodes.handle_from_index(i));
                            node.set_inv_bind_pose_transform(*resource_node.get_inv_bind_pose_transform());
                            break;
                        }
                    }
                }
            }
        }

        for animation in self.animations.iter_mut() {
            animation.resolve(&self.nodes)
        }

        // Then iterate over all scenes and resolve changes in surface data, remap bones, etc.
        // This step is needed to take correct graphical data from resource, we do not store
        // meshes in save files, just references to resource this data was taken from. So on
        // resolve stage we just copying surface from resource, do bones remapping. Bones remapping
        // is required stage because we copied surface from resource and bones are mapped to nodes
        // in resource, but we must have them mapped to instantiated nodes on scene. To do that
        // we'll try to find a root for each node, and starting from it we'll find corresponding
        // bone nodes. I know that this sounds too confusing but try to understand it.
        for i in 0..self.get_nodes().get_capacity() {
            let node_handle = self.get_nodes().handle_from_index(i);

            // TODO HACK: Fool borrow checker for now.
            let mscene = unsafe { &mut *(self as *mut Scene) };

            let root_handle = self.find_model_root(node_handle);

            if let Some(node) = self.get_nodes_mut().at_mut(i) {
                let node_name = String::from(node.get_name());
                if let Some(model) = node.get_resource() {
                    if let NodeKind::Mesh(mesh) = node.get_kind_mut() {
                        let model = model.lock().unwrap();
                        let resource_node_handle = model.find_node_by_name(node_name.as_str());
                        if let Some(resource_node) = model.get_scene().get_node(resource_node_handle) {
                            if let NodeKind::Mesh(resource_mesh) = resource_node.get_kind() {
                                // Copy surfaces from resource and assign to meshes.
                                let surfaces = mesh.get_surfaces_mut();
                                surfaces.clear();
                                for resource_surface in resource_mesh.get_surfaces() {
                                    surfaces.push(resource_surface.make_copy());
                                }

                                // Remap bones
                                for surface in mesh.get_surfaces_mut() {
                                    for bone_handle in surface.bones.iter_mut() {
                                        *bone_handle = mscene.find_copy_of(root_handle, *bone_handle);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update_nodes(&mut self) {
        // Calculate transforms on nodes
        self.stack.clear();
        self.stack.push(self.root);
        while let Some(handle) = self.stack.pop() {
            // Calculate local transform and get parent handle
            let mut parent_handle: Handle<Node> = Handle::none();
            if let Some(node) = self.nodes.borrow_mut(handle) {
                parent_handle = node.parent;
            }

            // Extract parent's global transform
            let mut parent_global_transform = Mat4::identity();
            let mut parent_visibility = true;
            if let Some(parent) = self.nodes.borrow(parent_handle) {
                parent_global_transform = parent.global_transform;
                parent_visibility = parent.global_visibility;
            }

            if let Some(node) = self.nodes.borrow_mut(handle) {
                node.global_transform = parent_global_transform * node.local_transform.get_matrix();
                node.global_visibility = parent_visibility && node.visibility;

                // Queue children and continue traversal on them
                for child_handle in node.children.iter() {
                    self.stack.push(child_handle.clone());
                }
            }
        }
    }

    pub fn update_animations(&mut self, dt: f32) {
        // Reset local transform of animated nodes first
        for animation in self.animations.iter() {
            for track in animation.get_tracks() {
                if let Some(node) = self.nodes.borrow_mut(track.get_node()) {
                    let transform = node.get_local_transform_mut();
                    transform.set_position(Default::default());
                    transform.set_rotation(Default::default());
                    // TODO: transform.set_scale(Vec3::make(1.0, 1.0, 1.0));
                }
            }
        }

        // Then apply animation.
        for animation in self.animations.iter_mut() {
            if !animation.is_enabled() {
                continue;
            }

            let next_time_pos = animation.get_time_position() + dt * animation.get_speed();

            let weight = animation.get_weight();

            for track in animation.get_tracks() {
                if !track.is_enabled() {
                    continue;
                }

                if let Some(keyframe) = track.get_key_frame(animation.get_time_position()) {
                    if let Some(node) = self.nodes.borrow_mut(track.get_node()) {
                        let transform = node.get_local_transform_mut();
                        transform.set_rotation(transform.get_rotation().nlerp(&keyframe.rotation, weight));
                        transform.set_position(transform.get_position() + keyframe.position.scale(weight));
                        // TODO: transform.set_scale(transform.get_scale().lerp(&keyframe.scale, weight));
                    }
                }
            }

            animation.set_time_position(next_time_pos);
            animation.update_fading(dt);
        }
    }

    pub fn update(&mut self, aspect_ratio: f32, dt: f32) {
        self.update_physics(dt);

        self.update_animations(dt);
        self.update_nodes();

        // HACK
        let camera_position = if let Some(camera) = self.nodes.borrow(self.active_camera) {
            camera.get_global_position()
        } else {
            Vec3::zero()
        };

        for node in self.nodes.iter_mut() {
            let eye = node.get_global_position();
            let look = node.get_look_vector();
            let up = node.get_up_vector();

            match node.get_kind_mut() {
                NodeKind::Camera(camera) => camera.calculate_matrices(eye, look, up, aspect_ratio),
                NodeKind::ParticleSystem(particle_system) => particle_system.update(dt, &eye, &camera_position),
                _ => ()
            }
        }
    }
}

impl Visit for Scene {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        self.root.visit("Root", visitor)?;
        self.active_camera.visit("ActiveCamera", visitor)?;
        self.nodes.visit("Nodes", visitor)?;
        self.animations.visit("Animations", visitor)?;
        self.physics.visit("Physics", visitor)?;
        visitor.leave_region()
    }
}
