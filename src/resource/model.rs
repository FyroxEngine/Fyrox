use crate::{
    scene::{
        Scene,
        node::Node
    },
    utils::pool::Handle,
    engine::State,
    resource::fbx
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

    pub fn instantiate(&self, state: &State, dest_scene: &mut Scene) -> Handle<Node> {
        return self.scene.copy_node(&self.scene.get_root(), state, dest_scene);
    }

    pub fn get_scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }
}