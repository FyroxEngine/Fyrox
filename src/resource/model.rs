use crate::{
    scene::{Scene, node::Node},
    utils::pool::Handle,
    engine::state::State,
    resource::{
        fbx,
        Resource,
        ResourceKind,
        fbx::error::FbxError,
    },
};
use std::{
    path::Path,
    cell::RefCell,
    rc::Rc,
};

pub struct Model {
    scene: Scene,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            scene: Scene::new(),
        }
    }
}

impl Model {
    pub fn load(path: &Path, state: &mut State) -> Result<Model, FbxError> {
        let mut scene = Scene::new();
        fbx::load_to_scene(&mut scene, state, path)?;
        Ok(Model { scene })
    }

    /// Tries to instantiate model from given resource. Returns non-none handle on success.
    pub fn instantiate(resource_rc: Rc<RefCell<Resource>>, dest_scene: &mut Scene) -> Result<Handle<Node>, ()> {
        let resource = resource_rc.borrow();
        if let ResourceKind::Model(model) = resource.borrow_kind() {
            let root = model.scene.copy_node(model.scene.get_root(), dest_scene);

            // Notify instantiated nodes about resource they were created from.
            let mut stack = Vec::new();
            stack.push(root);
            while let Some(node_handle) = stack.pop() {
                if let Some(node) = dest_scene.get_nodes_mut().borrow_mut(node_handle) {
                    node.set_resource(Rc::clone(&resource_rc));
                    // Continue on children.
                    for child_handle in node.get_children() {
                        stack.push(child_handle.clone());
                    }
                }
            }

            // Instantiate animations
            for ref_anim in model.scene.get_animations().iter() {
                let mut anim_copy = ref_anim.clone();

                // Remap animation track nodes.
                for (i, ref_track) in ref_anim.get_tracks().iter().enumerate() {
                    // Find instantiated node that corresponds to node in resource
                    let nodes = dest_scene.get_nodes();
                    for k in 0..nodes.get_capacity() {
                        if let Some(node) = nodes.at(k) {
                            if node.get_original_handle() == ref_track.get_node() {
                                anim_copy.get_tracks_mut()[i].set_node(nodes.handle_from_index(k));
                            }
                        }
                    }
                }

                dest_scene.add_animation(anim_copy);
            }

            return Ok(root);
        }
        Err(())
    }

    pub fn get_scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn get_scene(&self) -> &Scene {
        &self.scene
    }

    pub fn find_node_by_name(&self, name: &str) -> Handle<Node> {
        self.scene.find_node_by_name(self.scene.get_root(), name)
    }
}