//! Contains all structures and methods to create and manage scene graph nodes.
//!
//! Node is enumeration of possible types of scene nodes.

use crate::{
    core::define_is_as,
    core::visitor::{Visit, VisitResult, Visitor},
    scene::{
        base::Base, camera::Camera, light::Light, mesh::Mesh, particle_system::ParticleSystem,
        sprite::Sprite,
    },
};
use std::ops::{Deref, DerefMut};

/// Helper macros to reduce code bloat - its purpose it to dispatch
/// specified call by actual enum variant.
macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            Node::Base(v) => v.$func($($args),*),
            Node::Mesh(v) => v.$func($($args),*),
            Node::Camera(v) => v.$func($($args),*),
            Node::Light(v) => v.$func($($args),*),
            Node::ParticleSystem(v) => v.$func($($args),*),
            Node::Sprite(v) => v.$func($($args),*),
        }
    };
}

impl Visit for Node {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut kind_id = self.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            *self = Node::from_id(kind_id)?;
        }

        static_dispatch!(self, visit, name, visitor)
    }
}

/// See module docs.
#[derive(Clone, Debug)]
pub enum Node {
    /// See Base node docs.
    Base(Base),
    /// See Light node docs.
    Light(Light),
    /// See Camera node docs.
    Camera(Camera),
    /// See Mesh node docs.
    Mesh(Mesh),
    /// See Sprite node docs.
    Sprite(Sprite),
    /// See ParticleSystem node docs.
    ParticleSystem(ParticleSystem),
}

macro_rules! static_dispatch_deref {
    ($self:ident) => {
        match $self {
            Node::Base(v) => v,
            Node::Mesh(v) => v,
            Node::Camera(v) => v,
            Node::Light(v) => v,
            Node::ParticleSystem(v) => v,
            Node::Sprite(v) => v,
        }
    };
}

impl Deref for Node {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        static_dispatch_deref!(self)
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        static_dispatch_deref!(self)
    }
}

impl Default for Node {
    fn default() -> Self {
        Node::Base(Default::default())
    }
}

impl Node {
    /// Creates new Node based on variant id.
    pub fn from_id(id: u8) -> Result<Self, String> {
        match id {
            0 => Ok(Node::Base(Default::default())),
            1 => Ok(Node::Light(Default::default())),
            2 => Ok(Node::Camera(Default::default())),
            3 => Ok(Node::Mesh(Default::default())),
            4 => Ok(Node::Sprite(Default::default())),
            5 => Ok(Node::ParticleSystem(Default::default())),
            _ => Err(format!("Invalid node kind {}", id)),
        }
    }

    /// Returns actual variant id.
    pub fn id(&self) -> u8 {
        match self {
            Node::Base(_) => 0,
            Node::Light(_) => 1,
            Node::Camera(_) => 2,
            Node::Mesh(_) => 3,
            Node::Sprite(_) => 4,
            Node::ParticleSystem(_) => 5,
        }
    }

    define_is_as!(Node : Mesh -> ref Mesh => fn is_mesh, fn as_mesh, fn as_mesh_mut);
    define_is_as!(Node : Camera -> ref Camera => fn is_camera, fn as_camera, fn as_camera_mut);
    define_is_as!(Node : Light -> ref Light => fn is_light, fn as_light, fn as_light_mut);
    define_is_as!(Node : ParticleSystem -> ref ParticleSystem => fn is_particle_system, fn as_particle_system, fn as_particle_system_mut);
    define_is_as!(Node : Sprite -> ref Sprite => fn is_sprite, fn as_sprite, fn as_sprite_mut);
}
