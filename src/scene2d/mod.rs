//! Contains all structures and methods to create and manage 2D scenes.
//!
//! A `Scene` is a container for graph nodes, animations and physics.

use crate::physics2d::PhysicsPerformanceStatistics;
use crate::physics2d::RigidBodyHandle;
use crate::{
    core::{
        algebra::{Isometry2, Translation2, Vector2},
        color::Color,
        instant,
        pool::{Handle, Pool},
        visitor::prelude::*,
    },
    engine::PhysicsBinder,
    resource::texture::Texture,
    scene::base::PhysicsBinding,
    scene2d::{graph::Graph, node::Node, physics::Physics},
    sound::{context::SoundContext, engine::SoundEngine},
};
use std::collections::HashMap;
use std::{
    ops::{Index, IndexMut},
    sync::{Arc, Mutex},
};

pub mod base;
pub mod camera;
pub mod graph;
pub mod light;
pub mod node;
pub mod physics;
pub mod sprite;
pub mod transform;

/// A structure that holds times that specific update step took.
#[derive(Clone, Default, Debug)]
pub struct PerformanceStatistics {
    /// Physics performance statistics.
    pub physics: PhysicsPerformanceStatistics,

    /// A time (in seconds) which was required to update graph.
    pub graph_update_time: f32,

    /// A time (in seconds) which was required to render sounds.
    pub sound_update_time: f32,
}

#[derive(Visit, Default)]
pub struct Scene2d {
    pub graph: Graph,

    pub render_target: Option<Texture>,

    pub enabled: bool,

    pub ambient_light_color: Color,

    /// A sound context that holds all sound sources, effects, etc. belonging to the scene.
    pub sound_context: SoundContext,

    pub physics: Physics,

    pub physics_binder: PhysicsBinder<Node, RigidBodyHandle>,

    #[visit(skip)]
    pub performance_statistics: PerformanceStatistics,
}

impl Scene2d {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            render_target: None,
            enabled: true,
            ambient_light_color: Color::opaque(80, 80, 80),
            sound_context: SoundContext::new(),
            physics: Default::default(),
            physics_binder: Default::default(),
            performance_statistics: Default::default(),
        }
    }

    fn update_physics(&mut self) {
        self.physics.step();

        self.performance_statistics.physics = self.physics.performance_statistics.clone();
        self.physics.performance_statistics.reset();

        // Keep pair when node and body are both alive.
        let graph = &mut self.graph;
        let physics = &mut self.physics;
        self.physics_binder
            .retain(|node, body| graph.is_valid_handle(*node) && physics.bodies.contains(body));

        // Sync node positions with assigned physics bodies
        if self.physics_binder.enabled {
            for (&node_handle, body) in self.physics_binder.forward_map().iter() {
                let body = physics.bodies.get_mut(body).unwrap();
                let node = &mut self.graph[node_handle];
                match node.physics_binding {
                    PhysicsBinding::NodeWithBody => {
                        node.local_transform_mut()
                            .set_position(body.position().translation.vector)
                            .set_rotation(body.position().rotation.to_polar().1);
                    }
                    PhysicsBinding::BodyWithNode => {
                        let (r, p) = self.graph.isometric_global_rotation_position(node_handle);
                        body.set_position(
                            Isometry2 {
                                rotation: r,
                                translation: Translation2 { vector: p },
                            },
                            true,
                        );
                    }
                }
            }
        }
    }

    pub fn update(&mut self, render_target_size: Vector2<f32>, dt: f32) {
        self.update_physics();

        let last = instant::Instant::now();
        self.graph.update(render_target_size, dt);
        self.performance_statistics.graph_update_time =
            (instant::Instant::now() - last).as_secs_f32();

        self.performance_statistics.sound_update_time = self
            .sound_context
            .state()
            .full_render_duration()
            .as_secs_f32();
    }

    pub(in crate) fn resolve(&mut self) {
        self.physics.resolve();
    }

    /// Creates deep copy of a scene, filter predicate allows you to filter out nodes
    /// by your criteria.
    pub fn clone<F>(&self, filter: &mut F) -> (Self, HashMap<Handle<Node>, Handle<Node>>)
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let (graph, old_new_map) = self.graph.clone(filter);

        // It is ok to use old binder here, because handles maps one-to-one.
        let physics = self.physics.deep_copy();
        let mut physics_binder = PhysicsBinder::default();
        for (node, &body) in self.physics_binder.forward_map().iter() {
            // Make sure we bind existing node with new physical body.
            if let Some(&new_node) = old_new_map.get(node) {
                // Re-use of body handle is fine here because physics copy bodies
                // directly and handles from previous pool is still suitable for copy.
                physics_binder.bind(new_node, body);
            }
        }
        (
            Self {
                graph,
                physics,
                physics_binder,
                // Render target is intentionally not copied, because it does not makes sense - a copy
                // will redraw frame completely.
                render_target: Default::default(),
                sound_context: self.sound_context.deep_clone(),
                performance_statistics: Default::default(),
                ambient_light_color: self.ambient_light_color,
                enabled: self.enabled,
            },
            old_new_map,
        )
    }
}

#[derive(Visit)]
pub struct Scene2dContainer {
    pool: Pool<Scene2d>,
    sound_engine: Arc<Mutex<SoundEngine>>,
}

impl Scene2dContainer {
    pub fn new(sound_engine: Arc<Mutex<SoundEngine>>) -> Self {
        Self {
            pool: Default::default(),
            sound_engine,
        }
    }

    pub fn add(&mut self, scene: Scene2d) -> Handle<Scene2d> {
        self.sound_engine
            .lock()
            .unwrap()
            .add_context(scene.sound_context.clone());

        self.pool.spawn(scene)
    }

    pub fn remove(&mut self, scene_handle: Handle<Scene2d>) -> Scene2d {
        self.sound_engine
            .lock()
            .unwrap()
            .remove_context(self.pool[scene_handle].sound_context.clone());

        self.pool.free(scene_handle)
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

    pub fn pair_iter_mut(&mut self) -> impl Iterator<Item = (Handle<Scene2d>, &mut Scene2d)> {
        self.pool.pair_iter_mut()
    }

    pub fn clear(&mut self) {
        self.pool.clear();
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
