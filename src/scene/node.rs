use crate::core::{
    visitor::{Visit, VisitResult, Visitor},
};
use crate::scene::{
    light::Light,
    camera::Camera,
    base::{Base, AsBase},
    mesh::Mesh,
    sprite::Sprite,
    particle_system::ParticleSystem,
};

impl Visit for Node {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut kind_id = self.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            *self = Node::from_id(kind_id)?;
        }

        // Call appropriate visit method based on actual variant.
        match self {
            Node::Base(base) => base.visit(name, visitor),
            Node::Light(light) => light.visit(name, visitor),
            Node::Camera(camera) => camera.visit(name, visitor),
            Node::Mesh(mesh) => mesh.visit(name, visitor),
            Node::Sprite(sprite) => sprite.visit(name, visitor),
            Node::ParticleSystem(particle_system) => particle_system.visit(name, visitor)
        }
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

/// Helper macros to reduce code bloat - its purpose it to dispatch
/// specified call by actual enum variant.
macro_rules! dispatch {
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

impl AsBase for Node {
    fn base(&self) -> &Base {
        dispatch!(self, base, )
    }

    fn base_mut(&mut self) -> &mut Base {
        dispatch!(self, base_mut, )
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