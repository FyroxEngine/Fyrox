use crate::{
    scene::{
        Scene,
        node::Node
    },
    utils::pool::Handle,
    engine::State,
    resource::{
        fbx,
        Resource,
        ResourceKind
    },
    utils::rcpool::RcHandle,
};
use std::path::Path;

pub struct Model {
    scene: Scene,
}

impl Model {
    pub fn load(path: &Path, state: &mut State) -> Result<Model, String> {
        let mut scene = Scene::new();
        fbx::load_to_scene(&mut scene, state, path)?;
        Ok(Model { scene })
    }

    /// Tries to instantiate model from given resource. Returns non-none handle on success.
    pub fn instantiate(resource_handle: &RcHandle<Resource>, state: &State, dest_scene: &mut Scene) -> Result<Handle<Node>, ()> {
        if let Some(resource) = state.get_resource_manager().borrow_resource(resource_handle) {
            if let ResourceKind::Model(model) = resource.borrow_kind() {
                let root = model.scene.copy_node(&model.scene.get_root(), state, dest_scene);

                // Notify instantiated nodes about resource they were created from.
                let mut stack = Vec::new();
                stack.push(root.clone());
                while let Some(node_handle) = stack.pop() {
                    if let Some(node) = dest_scene.nodes.borrow_mut(&node_handle) {
                        node.set_resource(state.get_resource_manager().share_resource_handle(resource_handle));
                        for child_handle in node.children.iter() {
                            stack.push(child_handle.clone());
                        }
                    }
                }
                return Ok(root);
            }
        }
        Err(())
    }

    pub fn get_scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }
}