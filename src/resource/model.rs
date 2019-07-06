use crate::scene::Scene;
use crate::utils::pool::Handle;
use crate::scene::node::Node;
use crate::engine::State;
use std::path::Path;
use crate::resource::fbx;

pub struct Model {
    scene: Scene
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

    pub fn get_scene(&self) -> &Scene {
        &self.scene
    }
}