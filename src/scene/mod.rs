pub mod node;
pub mod animation;
pub mod mesh;
pub mod camera;
pub mod light;
pub mod particle_system;
pub mod transform;
pub mod sprite;
pub mod graph;

use crate::scene::{
    animation::AnimationContainer,
    graph::Graph,
    node::Node,
};
use rg3d_physics::{Physics, rigid_body::RigidBody};
use rg3d_core::{
    visitor::{Visit, VisitResult, Visitor},
    pool::Handle,
};
use std::collections::HashMap;
use rg3d_core::pool::{Pool, PoolIterator, PoolIteratorMut};

pub struct Scene {
    graph: Graph,
    animations: AnimationContainer,
    physics: Physics,
    node_rigid_body_map: HashMap<Handle<Node>, Handle<RigidBody>>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            animations: Default::default(),
            physics: Default::default(),
            node_rigid_body_map: Default::default()
        }
    }
}

pub struct SceneInterface<'a> {
    pub graph: &'a Graph,
    pub physics: &'a Physics,
    pub animations: &'a AnimationContainer,
    pub node_rigid_body_map: &'a HashMap<Handle<Node>, Handle<RigidBody>>,
}

pub struct SceneInterfaceMut<'a> {
    pub graph: &'a mut Graph,
    pub physics: &'a mut Physics,
    pub animations: &'a mut AnimationContainer,
    pub node_rigid_body_map: &'a mut HashMap<Handle<Node>, Handle<RigidBody>>,
}

impl Scene {
    #[inline]
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            physics: Physics::new(),
            animations: AnimationContainer::new(),
            node_rigid_body_map: HashMap::new(),
        }
    }

    pub fn interface(&self) -> SceneInterface {
        SceneInterface {
            graph: &self.graph,
            physics: &self.physics,
            animations: &self.animations,
            node_rigid_body_map: &self.node_rigid_body_map
        }
    }

    pub fn interface_mut(&mut self) -> SceneInterfaceMut {
        SceneInterfaceMut {
            graph: &mut self.graph,
            physics: &mut self.physics,
            animations: &mut self.animations,
            node_rigid_body_map: &mut self.node_rigid_body_map
        }
    }

    pub fn update_physics(&mut self, dt: f32) {
        self.physics.step(dt);

        // Sync node positions with assigned physics bodies
        for (node, body) in self.node_rigid_body_map.iter() {
            if let Some(node) = self.graph.get_mut(*node) {
                if let Some(body) = self.physics.borrow_body(*body) {
                    node.get_local_transform_mut().set_position(body.get_position());
                }
            }
        }
    }

    pub fn resolve(&mut self) {
        println!("Starting resolve...");
        self.graph.resolve();
        self.animations.resolve(&self.graph);
        println!("Resolve succeeded!");
    }

    pub fn update(&mut self, aspect_ratio: f32, dt: f32) {
        self.update_physics(dt);
        self.animations.update_animations(dt, &mut self.graph);
        self.graph.update_nodes(aspect_ratio, dt);
    }
}

impl Visit for Scene {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;
        self.node_rigid_body_map.visit("BodyMap", visitor)?;
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
    pub fn get(&self, handle: Handle<Scene>) -> Option<&Scene> {
        self.pool.borrow(handle)
    }

    #[inline]
    pub fn get_mut(&mut self, handle: Handle<Scene>) -> Option<&mut Scene> {
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