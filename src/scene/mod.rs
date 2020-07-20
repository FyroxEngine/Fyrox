#![warn(missing_docs)]

//! Contains all structures and methods to create and manage scenes.
//!
//! Scene is container for graph nodes, animations and physics.

pub mod base;
pub mod camera;
pub mod graph;
pub mod light;
pub mod mesh;
pub mod node;
pub mod particle_system;
pub mod sprite;
pub mod transform;

use crate::{
    animation::AnimationContainer,
    core::{
        math::vec2::Vec2,
        pool::{Handle, Pool, PoolIterator, PoolIteratorMut},
        visitor::{Visit, VisitResult, Visitor},
    },
    physics::{rigid_body::RigidBody, Physics},
    resource::texture::Texture,
    scene::{graph::Graph, node::Node},
    utils::log::Log,
};
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
    sync::{Arc, Mutex},
};

/// Physics binder is used to link graph nodes with rigid bodies. Scene will
/// sync transform of node with its associated rigid body.
#[derive(Clone, Debug)]
pub struct PhysicsBinder {
    node_rigid_body_map: HashMap<Handle<Node>, Handle<RigidBody>>,
}

impl Default for PhysicsBinder {
    fn default() -> Self {
        Self {
            node_rigid_body_map: Default::default(),
        }
    }
}

impl PhysicsBinder {
    /// Links given graph node with specified rigid body.
    pub fn bind(
        &mut self,
        node: Handle<Node>,
        rigid_body: Handle<RigidBody>,
    ) -> Option<Handle<RigidBody>> {
        self.node_rigid_body_map.insert(node, rigid_body)
    }

    /// Unlinks given graph node from its associated rigid body (if any).
    pub fn unbind(&mut self, node: Handle<Node>) -> Option<Handle<RigidBody>> {
        self.node_rigid_body_map.remove(&node)
    }

    /// Returns handle of rigid body associated with given node. It will return
    /// Handle::NONE if given node isn't linked to a rigid body.
    pub fn body_of(&self, node: Handle<Node>) -> Handle<RigidBody> {
        self.node_rigid_body_map
            .get(&node)
            .map(|b| *b)
            .unwrap_or_default()
    }
}

impl Visit for PhysicsBinder {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.node_rigid_body_map.visit("Map", visitor)?;

        visitor.leave_region()
    }
}

/// See module docs.
#[derive(Debug)]
pub struct Scene {
    /// Graph is main container for all scene nodes. It calculates global transforms for nodes,
    /// updates them and performs all other important work. See `graph` module docs for more
    /// info.
    pub graph: Graph,

    /// Animations container controls all animation on scene. Each animation can have tracks which
    /// has handles to graph nodes. See `animation` module docs for more info.
    pub animations: AnimationContainer,

    /// Physics world. Allows you create various physics objects such as static geometries and
    /// rigid bodies. Rigid bodies then should be linked with graph nodes using binder.
    pub physics: Physics,

    /// Physics binder is a bridge between physics world and scene graph. If a rigid body is linked
    /// to a graph node, then rigid body will control local transform of node.
    pub physics_binder: PhysicsBinder,

    /// Texture to draw scene to. If empty, scene will be drawn on screen directly.
    /// It is useful to "embed" some scene into other by drawing a quad with this
    /// texture. This can be used to make in-game video conference - you can make
    /// separate scene with your characters and draw scene into texture, then in
    /// main scene you can attach this texture to some quad which will be used as
    /// monitor. Other usage could be previewer of models, like pictogram of character
    /// in real-time strategies, in other words there are plenty of possible uses.
    pub render_target: Option<Arc<Mutex<Texture>>>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            animations: Default::default(),
            physics: Default::default(),
            physics_binder: Default::default(),
            render_target: None,
        }
    }
}

impl Scene {
    /// Creates new scene with single root node.
    ///
    /// # Notes
    ///
    /// This method differs from Default trait implementation! Scene::default() creates
    /// empty graph with no nodes.
    #[inline]
    pub fn new() -> Self {
        Self {
            // Graph must be created with `new` method because it differs from `default`
            graph: Graph::new(),
            physics: Default::default(),
            animations: Default::default(),
            physics_binder: Default::default(),
            render_target: None,
        }
    }

    fn update_physics(&mut self, dt: f32) {
        self.physics.step(dt);

        // Keep pair when node and body are both alive.
        let graph = &self.graph;
        let physics = &self.physics;
        self.physics_binder
            .node_rigid_body_map
            .retain(|node, body| {
                graph.is_valid_handle(*node) && physics.is_valid_body_handle(*body)
            });

        // Sync node positions with assigned physics bodies
        for (node, body) in self.physics_binder.node_rigid_body_map.iter() {
            let body = physics.borrow_body(*body);
            self.graph[*node]
                .local_transform_mut()
                .set_position(body.get_position());
        }
    }

    /// Removes node from scene with all associated entities, like animations etc.
    ///
    /// # Panics
    ///
    /// Panics if handle is invalid.
    pub fn remove_node(&mut self, handle: Handle<Node>) {
        for descendant in self.graph.traverse_handle_iter(handle) {
            // Remove all associated animations.
            self.animations.retain(|animation| {
                for track in animation.get_tracks() {
                    if track.get_node() == descendant {
                        return false;
                    }
                }
                true
            });
        }

        self.graph.remove_node(handle)
    }

    pub(in crate) fn resolve(&mut self) {
        Log::writeln("Starting resolve...".to_owned());
        self.graph.resolve();
        self.animations.resolve(&self.graph);
        Log::writeln("Resolve succeeded!".to_owned());
    }

    /// Performs single update tick with given delta time from last frame. Internally
    /// it updates physics, animations, and each graph node. In most cases there is
    /// no need to call it directly, engine automatically updates all available scenes.
    pub fn update(&mut self, frame_size: Vec2, dt: f32) {
        self.update_physics(dt);
        self.animations.update_animations(dt);
        self.graph.update_nodes(frame_size, dt);
    }

    /// Creates deep copy of a scene, filter predicate allows you to filter out nodes
    /// by your criteria.
    pub fn clone<F>(&self, filter: &mut F) -> Self
    where
        F: FnMut(Handle<Node>, &Node) -> bool,
    {
        let (graph, old_new_map) = self.graph.clone(filter);
        let mut animations = self.animations.clone();
        for animation in animations.iter_mut() {
            // Remove all tracks for nodes that were filtered out.
            animation.retain_tracks(|track| old_new_map.contains_key(&track.get_node()));
            // Remap track nodes.
            for track in animation.get_tracks_mut() {
                track.set_node(old_new_map[&track.get_node()]);
            }
        }
        let physics = self.physics.clone();
        let mut physics_binder = PhysicsBinder::default();
        for (node, &body) in self.physics_binder.node_rigid_body_map.iter() {
            // Make sure we bind existing node with new physical body.
            if let Some(&new_node) = old_new_map.get(node) {
                // Re-use of body handle is fine here because physics copy bodies
                // directly and handles from previous pool is still suitable for copy.
                physics_binder.bind(new_node, body);
            }
        }
        Self {
            graph,
            animations,
            physics,
            physics_binder,
            render_target: Default::default(),
        }
    }
}

impl Visit for Scene {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        self.physics_binder.visit("PhysicsBinder", visitor)?;
        self.graph.visit("Graph", visitor)?;
        self.animations.visit("Animations", visitor)?;
        self.physics.visit("Physics", visitor)?;
        visitor.leave_region()
    }
}

/// Container for scenes in the engine. It just a simple wrapper around Pool.
pub struct SceneContainer {
    pool: Pool<Scene>,
}

impl SceneContainer {
    pub(in crate) fn new() -> Self {
        Self { pool: Pool::new() }
    }

    /// Creates new iterator over scenes in container.
    #[inline]
    pub fn iter(&self) -> PoolIterator<Scene> {
        self.pool.iter()
    }

    /// Creates new mutable iterator over scenes in container.
    #[inline]
    pub fn iter_mut(&mut self) -> PoolIteratorMut<Scene> {
        self.pool.iter_mut()
    }

    /// Adds new scene into container.
    #[inline]
    pub fn add(&mut self, scene: Scene) -> Handle<Scene> {
        self.pool.spawn(scene)
    }

    /// Removes all scenes from container.
    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    /// Removes given scene from container.
    #[inline]
    pub fn remove(&mut self, handle: Handle<Scene>) {
        self.pool.free(handle);
    }
}

impl Index<Handle<Scene>> for SceneContainer {
    type Output = Scene;

    #[inline]
    fn index(&self, index: Handle<Scene>) -> &Self::Output {
        &self.pool[index]
    }
}

impl IndexMut<Handle<Scene>> for SceneContainer {
    #[inline]
    fn index_mut(&mut self, index: Handle<Scene>) -> &mut Self::Output {
        &mut self.pool[index]
    }
}

impl Default for SceneContainer {
    fn default() -> Self {
        Self { pool: Pool::new() }
    }
}

impl Visit for SceneContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}
