use crate::{
    core::{
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
    resource::texture::Texture,
    scene2d::graph::Graph,
};

pub mod base;
pub mod camera;
pub mod graph;
pub mod light;
pub mod node;
pub mod sprite;
pub mod transform;

#[derive(Visit, Default)]
pub struct Scene2d {
    pub graph: Graph,

    pub render_target: Option<Texture>,
}

#[derive(Default, Visit)]
pub struct Scene2dContainer {
    pool: Pool<Scene2d>,
}

impl Scene2dContainer {
    pub fn add(&mut self, scene: Scene2d) -> Handle<Scene2d> {
        self.pool.spawn(scene)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Scene2d> {
        self.pool.iter()
    }

    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Scene2d>, &Scene2d)> {
        self.pool.pair_iter()
    }
}
