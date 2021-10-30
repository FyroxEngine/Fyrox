//! Contains all structures and methods to create and manage scene graph nodes.
//!
//! For more info see [`Node`]

#![warn(missing_docs)]

use crate::asset::core::inspect::PropertyInfo;
use crate::core::inspect::Inspect;
use crate::core::math::aabb::AxisAlignedBoundingBox;
use crate::{
    core::{
        define_is_as,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        base::Base, camera::Camera, decal::Decal, light::Light, mesh::Mesh,
        particle_system::ParticleSystem, sprite::Sprite, terrain::Terrain,
    },
};
use std::ops::{Deref, DerefMut};

/// Helper macros to reduce code bloat - its purpose it to dispatch specified call by
/// actual enum variant.
macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            Node::Base(v) => v.$func($($args),*),
            Node::Mesh(v) => v.$func($($args),*),
            Node::Camera(v) => v.$func($($args),*),
            Node::Light(v) => v.$func($($args),*),
            Node::ParticleSystem(v) => v.$func($($args),*),
            Node::Sprite(v) => v.$func($($args),*),
            Node::Terrain(v) => v.$func($($args),*),
            Node::Decal(v) => v.$func($($args),*),
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

impl Inspect for Node {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        static_dispatch!(self, properties,)
    }
}

/// Node is the basic building block for 3D scenes. It has multiple variants, but all of them share some
/// common functionality:
///
/// - Local and global [transform](super::transform::Transform)
/// - Info about connections with other nodes in scene
/// - Visibility state - local and global
/// - Name and tags
/// - Level of details
/// - Physics binding mode
///
/// The exact functionality depends on variant of the node, check the respective docs for a variant you
/// interested in.
///
/// # Hierarchy
///
/// Nodes can be connected with other nodes, so a child node will be moved/rotate/scaled together with parent
/// node. This has some analogy in real world - imagine a pen with a cap. The pen will be the parent node in
/// the hierarchy and the cap will be child node. When you moving the pen, the cap moves with it only if it
/// attached to the pen. The same principle works with scene nodes.
///
/// # Transform
///
/// The node has two kinds of transform - local and global. Local transform defines where the node is located
/// (translation) relative to origin, how much it is scaled (in percent) and rotated (around any arbitrary axis).
/// Global transform is almost the same, but it also includes the whole chain of transforms of parent nodes.
/// In the previous example with the pen, the cap has its own local transform which tells how much it should be
/// moved from origin to be exactly on top of the pen. But global transform of the cap includes transform of the
/// pen. So if you move the pen, the local transform of the cap will remain the same, but global transform will
/// include the transform of the pen.
///
/// # Name and tag
///
/// The node can have arbitrary name and tag. Both could be used to search the node in the graph. Unlike the name,
/// tag could be used to store some gameplay information about the node. For example you can place a [`Mesh`] node
/// that represents health pack model and it will have a name "HealthPack", in the tag you could put additional info
/// like "MediumPack", "SmallPack", etc. So 3D model will not have "garbage" in its name, it will be stored inside tag.
///
/// # Visibility
///
/// The now has two kinds of visibility - local and global. As with transform, everything here is pretty similar.
/// Local visibility defines if the node is visible as if it would be rendered alone, global visibility includes
/// the combined visibility of entire chain of parent nodes.
///
/// Please keep in mind that "visibility" here means some sort of a "switch" that tells the renderer whether to draw
/// the node or not. To fetch actual visibility of the node from a camera's perspective, use
/// [visibility cache](super::visibility::VisibilityCache) of the camera.
///
/// # Level of details
///
/// The node could control which children nodes should be drawn based on the distance to a camera, this is so called
/// level of detail functionality. There is a separate article about LODs, it can be found [here](super::base::LevelOfDetail).
#[derive(Debug)]
pub enum Node {
    /// A node that offers basic functionality, every other node shares this functionality.
    ///
    /// For more info see [`Base`] node docs.
    Base(Base),

    /// A node that represents various light sources.
    ///
    /// For more info see [`Light`] node docs.
    Light(Light),

    /// A node that could be described as "our eyes in the world", a scene should have at least one camera for you
    /// to be able to see anything.
    ///
    /// For more info see [`Camera`] node docs.
    Camera(Camera),

    /// A node that is used for any kind of 3D models.
    ///
    /// For more info see [`Mesh`] node docs.
    Mesh(Mesh),

    /// Special variation of [`Mesh`](Node::Mesh) variant which ensures that a rectangular face (billboard) is always rotated
    /// in way so it always faces the camera.
    ///
    /// For more info see [`Sprite`] node docs.
    Sprite(Sprite),

    /// Collections of particles that is used to simulate clouds of particles, usually it is used to simulate dust, smoke, sparks
    /// etc in scenes.
    ///
    /// For more info see [`ParticleSystem`] node docs.
    ParticleSystem(ParticleSystem),

    /// A heightmap with multiple layers.
    ///
    /// For more info see [`Terrain`] node docs.
    Terrain(Terrain),

    /// A node that paints on other nodes using a texture. It is used to simulate cracks in concrete walls, damaged parts of the road,
    /// blood splatters, bullet holes, etc.
    ///
    /// For more info see Decal node docs.
    Decal(Decal),
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
            Node::Terrain(v) => v,
            Node::Decal(v) => v,
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
        Self::Base(Default::default())
    }
}

impl Node {
    /// Returns axis-aligned bounding box in **local space** of the node.
    pub fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        static_dispatch!(self, local_bounding_box,)
    }

    /// Returns axis-aligned bounding box in **world space** of the node.
    pub fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        static_dispatch!(self, world_bounding_box,)
    }

    /// Creates new Node based on variant id.
    pub fn from_id(id: u8) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Base(Default::default())),
            1 => Ok(Self::Light(Default::default())),
            2 => Ok(Self::Camera(Default::default())),
            3 => Ok(Self::Mesh(Default::default())),
            4 => Ok(Self::Sprite(Default::default())),
            5 => Ok(Self::ParticleSystem(Default::default())),
            6 => Ok(Self::Terrain(Default::default())),
            7 => Ok(Self::Decal(Default::default())),
            _ => Err(format!("Invalid node kind {}", id)),
        }
    }

    /// Returns actual variant id.
    pub fn id(&self) -> u8 {
        match self {
            Self::Base(_) => 0,
            Self::Light(_) => 1,
            Self::Camera(_) => 2,
            Self::Mesh(_) => 3,
            Self::Sprite(_) => 4,
            Self::ParticleSystem(_) => 5,
            Self::Terrain(_) => 6,
            Self::Decal(_) => 7,
        }
    }

    /// This method creates raw copy of a node, it should never be called in normal circumstances
    /// because internally nodes may (and most likely will) contain handles to other nodes. To
    /// correctly clone a node you have to use [copy_node](struct.Graph.html#method.copy_node).
    pub fn raw_copy(&self) -> Self {
        match self {
            Node::Base(v) => Node::Base(v.raw_copy()),
            Node::Light(v) => Node::Light(v.raw_copy()),
            Node::Camera(v) => Node::Camera(v.raw_copy()),
            Node::Mesh(v) => Node::Mesh(v.raw_copy()),
            Node::Sprite(v) => Node::Sprite(v.raw_copy()),
            Node::ParticleSystem(v) => Node::ParticleSystem(v.raw_copy()),
            Node::Terrain(v) => Node::Terrain(v.raw_copy()),
            Node::Decal(v) => Node::Decal(v.raw_copy()),
        }
    }

    define_is_as!(Node : Mesh -> ref Mesh => fn is_mesh, fn as_mesh, fn as_mesh_mut);
    define_is_as!(Node : Camera -> ref Camera => fn is_camera, fn as_camera, fn as_camera_mut);
    define_is_as!(Node : Light -> ref Light => fn is_light, fn as_light, fn as_light_mut);
    define_is_as!(Node : ParticleSystem -> ref ParticleSystem => fn is_particle_system, fn as_particle_system, fn as_particle_system_mut);
    define_is_as!(Node : Sprite -> ref Sprite => fn is_sprite, fn as_sprite, fn as_sprite_mut);
    define_is_as!(Node : Terrain -> ref Terrain => fn is_terrain, fn as_terrain, fn as_terrain_mut);
    define_is_as!(Node : Decal -> ref Decal => fn is_decal, fn as_decal, fn as_decal_mut);
}
