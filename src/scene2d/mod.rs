use crate::{
    core::{
        algebra::Vector2,
        color::Color,
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
    resource::texture::Texture,
    scene2d::graph::Graph,
};
use std::ops::{Index, IndexMut};

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

    pub enabled: bool,

    pub ambient_light_color: Color,
}

impl Scene2d {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            render_target: None,
            enabled: true,
            ambient_light_color: Color::opaque(80, 80, 80),
        }
    }

    pub fn update(&mut self, render_target_size: Vector2<f32>, dt: f32) {
        self.graph.update(render_target_size, dt);
    }
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

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Scene2d> {
        self.pool.iter_mut()
    }

    pub fn pair_iter(&self) -> impl Iterator<Item = (Handle<Scene2d>, &Scene2d)> {
        self.pool.pair_iter()
    }
}

impl Index<Handle<Scene2d>> for Scene2dContainer {
    type Output = Scene2d;

    fn index(&self, index: Handle<Scene2d>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Scene2d>> for Scene2dContainer {
    fn index_mut(&mut self, index: Handle<Scene2d>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}
