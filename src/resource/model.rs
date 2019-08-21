use crate::{
    scene::{
        Scene,
        node::Node,
    },
    utils::pool::Handle,
    engine::State,
    resource::{
        fbx,
        Resource,
        ResourceKind,
    },
};
use std::{
    path::Path,
    cell::RefCell,
    rc::Rc
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
    pub fn load(path: &Path, state: &mut State) -> Result<Model, String> {
        let mut scene = Scene::new();
        fbx::load_to_scene(&mut scene, state, path)?;
        Ok(Model { scene })
    }

    /// Tries to instantiate model from given resource. Returns non-none handle on success.
    pub fn instantiate(resource_rc: Rc<RefCell<Resource>>, dest_scene: &mut Scene) -> Result<Handle<Node>, ()> {
        let resource = resource_rc.borrow();
        if let ResourceKind::Model(model) = resource.borrow_kind() {
            let root = model.scene.copy_node(&model.scene.get_root(), dest_scene);

            // Notify instantiated nodes about resource they were created from.
            let mut stack = Vec::new();
            stack.push(root.clone());
            while let Some(node_handle) = stack.pop() {
                if let Some(node) = dest_scene.nodes.borrow_mut(&node_handle) {
                    node.set_resource(Rc::clone(&resource_rc));
                    for child_handle in node.get_children() {
                        stack.push(child_handle.clone());
                    }
                }
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
        self.scene.find_node_by_name(&self.scene.get_root(), name)
    }
}