use crate::{
    core::{
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        light::Light,
        camera::Camera,
        mesh::Mesh,
        sprite::Sprite,
        particle_system::ParticleSystem,
        base::Base
    }
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

#[derive(Clone)]
pub enum Node {
    Base(Base),
    Light(Light),
    Camera(Camera),
    Mesh(Mesh),
    Sprite(Sprite),
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

/// Defines as_(variant), as_mut_(variant) and is_(variant) methods.
macro_rules! define_is_as {
    ($is:ident, $as_ref:ident, $as_mut:ident, $kind:ident, $result:ty) => {
        pub fn $is(&self) -> bool {
            match self {
                Node::$kind(_) => true,
                _ => false
            }
        }

        pub fn $as_ref(&self) -> &$result {
            match self {
                Node::$kind(ref val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }

        pub fn $as_mut(&mut self) -> &mut $result {
            match self {
                Node::$kind(ref mut val) => val,
                _ => panic!("Cast to {} failed!", stringify!($kind))
            }
        }
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
            _ => Err(format!("Invalid node kind {}", id))
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

    define_is_as!(is_mesh, as_mesh, as_mesh_mut, Mesh, Mesh);
    define_is_as!(is_camera, as_camera, as_camera_mut, Camera, Camera);
    define_is_as!(is_light, as_light, as_light_mut, Light, Light);
    define_is_as!(is_particle_system, as_particle_system, as_particle_system_mut, ParticleSystem, ParticleSystem);
    define_is_as!(is_sprite, as_sprite, as_sprite_mut, Sprite, Sprite);
}