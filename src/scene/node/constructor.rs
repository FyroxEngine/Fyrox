//! A special container that is able to create nodes by their type UUID.

use crate::{
    core::{parking_lot::Mutex, uuid::Uuid, TypeUuidProvider},
    scene::{
        self,
        animation::{absm::AnimationBlendingStateMachine, AnimationPlayer},
        camera::Camera,
        decal::Decal,
        dim2::{self, rectangle::Rectangle},
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::Mesh,
        navmesh::NavigationalMesh,
        node::{Node, NodeTrait},
        particle_system::ParticleSystem,
        pivot::Pivot,
        ragdoll::Ragdoll,
        sound::{listener::Listener, Sound},
        sprite::Sprite,
        terrain::Terrain,
    },
};
use fxhash::FxHashMap;

/// A simple type alias for boxed node constructor.
pub type NodeConstructor = Box<dyn FnMut() -> Node + Send>;

/// A special container that is able to create nodes by their type UUID.
#[derive(Default)]
pub struct NodeConstructorContainer {
    map: Mutex<FxHashMap<Uuid, NodeConstructor>>,
}

impl NodeConstructorContainer {
    /// Creates default node constructor container with constructors for built-in engine nodes.
    pub fn new() -> Self {
        let container = NodeConstructorContainer::default();

        container.add::<dim2::collider::Collider>();
        container.add::<dim2::joint::Joint>();
        container.add::<Rectangle>();
        container.add::<dim2::rigidbody::RigidBody>();
        container.add::<DirectionalLight>();
        container.add::<PointLight>();
        container.add::<SpotLight>();
        container.add::<Mesh>();
        container.add::<ParticleSystem>();
        container.add::<Sound>();
        container.add::<Listener>();
        container.add::<Camera>();
        container.add::<scene::collider::Collider>();
        container.add::<Decal>();
        container.add::<scene::joint::Joint>();
        container.add::<Pivot>();
        container.add::<scene::rigidbody::RigidBody>();
        container.add::<Sprite>();
        container.add::<Terrain>();
        container.add::<AnimationPlayer>();
        container.add::<AnimationBlendingStateMachine>();
        container.add::<NavigationalMesh>();
        container.add::<Ragdoll>();

        container
    }

    /// Adds new type constructor for a given type and return previous constructor for the type
    /// (if any).
    pub fn add<T>(&self)
    where
        T: TypeUuidProvider + NodeTrait + Default,
    {
        let previous = self
            .map
            .lock()
            .insert(T::type_uuid(), Box::new(|| Node::new(T::default())));

        assert!(previous.is_none());
    }

    /// Adds custom type constructor.
    pub fn add_custom(&self, type_uuid: Uuid, constructor: NodeConstructor) {
        self.map.lock().insert(type_uuid, constructor);
    }

    /// Unregisters type constructor.
    pub fn remove(&self, type_uuid: Uuid) {
        self.map.lock().remove(&type_uuid);
    }

    /// Makes an attempt to create a node using provided type UUID. It may fail if there is no
    /// node constructor for specified type UUID.
    pub fn try_create(&self, type_uuid: &Uuid) -> Option<Node> {
        self.map.lock().get_mut(type_uuid).map(|c| (c)())
    }

    /// Returns total amount of constructors.
    pub fn len(&self) -> usize {
        self.map.lock().len()
    }

    /// Returns true if the container is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
