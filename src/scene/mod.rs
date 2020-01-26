#[macro_use]
pub mod node;
pub mod mesh;
pub mod camera;
pub mod light;
pub mod particle_system;
pub mod transform;
pub mod sprite;
pub mod graph;
pub mod base;

use crate::{
    core::{
        visitor::{Visit, VisitResult, Visitor},
        pool::{
            Handle,
            Pool,
            PoolIterator,
            PoolIteratorMut,
        },
    },
    physics::{
        Physics,
        rigid_body::RigidBody
    },
    scene::{
        graph::Graph,
        node::Node,
        base::AsBase,
    },
    animation::AnimationContainer,
    utils::log::Log
};
use std::collections::HashMap;

pub struct PhysicsBinder {
    node_rigid_body_map: HashMap<Handle<Node>, Handle<RigidBody>>
}

impl Default for PhysicsBinder {
    fn default() -> Self {
        Self {
            node_rigid_body_map: Default::default()
        }
    }
}

impl PhysicsBinder {
    pub fn bind(&mut self, node: Handle<Node>, rigid_body: Handle<RigidBody>) -> Option<Handle<RigidBody>> {
        self.node_rigid_body_map.insert(node, rigid_body)
    }

    pub fn unbind(&mut self, node: Handle<Node>) -> Option<Handle<RigidBody>> {
        self.node_rigid_body_map.remove(&node)
    }
}

impl Visit for PhysicsBinder {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.node_rigid_body_map.visit("Map", visitor)?;

        visitor.leave_region()
    }
}

pub struct Scene {
    pub graph: Graph,
    pub animations: AnimationContainer,
    pub physics: Physics,
    pub physics_binder: PhysicsBinder,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            animations: Default::default(),
            physics: Default::default(),
            physics_binder: Default::default(),
        }
    }
}

impl Scene {
    #[inline]
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            physics: Physics::new(),
            animations: AnimationContainer::new(),
            physics_binder: Default::default(),
        }
    }

    fn update_physics(&mut self, dt: f32) {
        self.physics.step(dt);

        // Sync node positions with assigned physics bodies
        for (node, body) in self.physics_binder.node_rigid_body_map.iter() {
            if self.graph.is_valid_handle(*node) {
                let node = self.graph.get_mut(*node).base_mut();
                if self.physics.is_valid_body_handle(*body) {
                    let body = self.physics.borrow_body(*body);
                    node.get_local_transform_mut().set_position(body.get_position());
                }
            }
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

    pub fn resolve(&mut self) {
        Log::writeln("Starting resolve...".to_owned());
        self.graph.resolve();
        self.animations.resolve(&self.graph);
        Log::writeln("Resolve succeeded!".to_owned());
    }

    pub fn update(&mut self, aspect_ratio: f32, dt: f32) {
        self.update_physics(dt);
        self.animations.update_animations(dt);
        self.graph.update_nodes(aspect_ratio, dt);

        // Keep pair when node and body are both alive.
        let graph = &self.graph;
        let physics = &self.physics;
        self.physics_binder.node_rigid_body_map.retain(|node, body| {
            graph.is_valid_handle(*node) && physics.is_valid_body_handle(*body)
        });
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

pub struct SceneContainer {
    pool: Pool<Scene>
}

impl SceneContainer {
    pub(in crate) fn new() -> Self {
        Self {
            pool: Pool::new()
        }
    }

    #[inline]
    pub fn iter(&self) -> PoolIterator<Scene> {
        self.pool.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> PoolIteratorMut<Scene> {
        self.pool.iter_mut()
    }

    #[inline]
    pub fn add(&mut self, animation: Scene) -> Handle<Scene> {
        self.pool.spawn(animation)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.pool.clear()
    }

    #[inline]
    pub fn remove(&mut self, handle: Handle<Scene>) {
        self.pool.free(handle)
    }

    #[inline]
    pub fn get(&self, handle: Handle<Scene>) -> &Scene {
        self.pool.borrow(handle)
    }

    #[inline]
    pub fn get_mut(&mut self, handle: Handle<Scene>) -> &mut Scene {
        self.pool.borrow_mut(handle)
    }
}

impl Default for SceneContainer {
    fn default() -> Self {
        Self {
            pool: Pool::new()
        }
    }
}

impl Visit for SceneContainer {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.pool.visit("Pool", visitor)?;

        visitor.leave_region()
    }
}