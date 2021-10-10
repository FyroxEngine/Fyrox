use crate::{physics::RigidBody, scene::EditorScene};
use rg3d::{core::pool::Handle, scene::graph::Graph};

pub mod item;
pub mod selection;

pub fn fetch_name(body: Handle<RigidBody>, editor_scene: &EditorScene, graph: &Graph) -> String {
    if let Some(associated_node) = editor_scene.physics.binder.backward_map().get(&body) {
        graph[*associated_node].name_owned()
    } else {
        "Rigid Body".to_string()
    }
}
