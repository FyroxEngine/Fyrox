use crate::core::pool::{Handle, Pool};
use crate::scene2d::graph::Graph;

pub mod base;
pub mod camera;
pub mod graph;
pub mod light;
pub mod node;
pub mod sprite;
pub mod transform;

pub struct Scene2d {
    pub graph: Graph,
}

pub struct Scene2dContainer {
    scenes: Pool<Scene2d>,
}

impl Scene2dContainer {
    pub fn add(&mut self, scene: Scene2d) -> Handle<Scene2d> {
        self.scenes.spawn(scene)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Scene2d> {
        self.scenes.iter()
    }
}
