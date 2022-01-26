//! Contains all structures and methods to create and manage sprites.
//!
//! For more info see [`Sprite`].

use crate::engine::resource_manager::ResourceManager;
use crate::{
    core::{
        color::Color,
        inspect::{Inspect, PropertyInfo},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

/// Sprite is billboard which always faces towards camera. It can be used as a "model" for bullets, and so on.
///
/// # Implementation details
///
/// Sprite is just a ready-to-use mesh with special material that ensures that the orientation of the faces
/// in the mesh is always on camera. Nothing stops you from implementing this manually using [Mesh](super::mesh::Mesh),
/// it could be done by using Forward render pass. You may need this for custom effects. Current implementation
/// is very simple, but still covers 95% of use cases.
///
/// # Depth sorting
///
/// Sprites are **not** depth-sorted so there could be some blending issues if multiple sprites are stacked one behind
/// another.
///
/// # Performance
///
/// Huge amount of sprites may cause performance issues, also you should not use sprites to make particle systems,
/// use [ParticleSystem](super::particle_system::ParticleSystem) instead.
///
/// # Example
///
/// ```rust
/// use fyrox::{
///     scene::{
///         node::Node,
///         sprite::SpriteBuilder,
///         base::BaseBuilder,
///         graph::Graph
///     },
///     engine::resource_manager::ResourceManager,
///     core::pool::{Handle},
/// };
///
/// fn create_smoke(resource_manager: ResourceManager, graph: &mut Graph) -> Handle<Node> {
///     SpriteBuilder::new(BaseBuilder::new())
///         .with_texture(resource_manager.request_texture("smoke.png"))
///         .build(graph)
/// }
/// ```
#[derive(Debug, Inspect)]
pub struct Sprite {
    base: Base,
    texture: Option<Texture>,
    color: Color,
    #[inspect(min_value = 0.0, step = 0.1)]
    size: f32,
    rotation: f32,
}

impl Deref for Sprite {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Sprite {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Default for Sprite {
    fn default() -> Self {
        SpriteBuilder::new(BaseBuilder::new()).build_sprite()
    }
}

impl Sprite {
    /// Creates a raw copy of a sprite node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            texture: self.texture.clone(),
            color: self.color,
            size: self.size,
            rotation: self.rotation,
        }
    }

    /// Sets new size of sprite. Since sprite is always square, size defines half of width or height, so actual size
    /// will be doubled. Default value is 0.2.    
    ///
    /// Negative values could be used to "inverse" the image on the sprite.
    pub fn set_size(&mut self, size: f32) {
        self.size = size;
    }

    /// Returns current size of sprite.
    pub fn size(&self) -> f32 {
        self.size
    }

    /// Sets new color of sprite. Default is White.
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Returns current color of sprite.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Sets rotation around "look" axis in radians. Default is 0.0.
    pub fn set_rotation(&mut self, rotation: f32) {
        self.rotation = rotation;
    }

    /// Returns rotation in radians.
    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    /// Sets new texture for sprite. Default is None.
    pub fn set_texture(&mut self, texture: Option<Texture>) {
        self.texture = texture;
    }

    /// Returns current texture of sprite. Can be None if sprite has no texture.
    pub fn texture(&self) -> Option<Texture> {
        self.texture.clone()
    }

    /// Returns current texture of sprite by ref. Can be None if sprite has no texture.
    pub fn texture_ref(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    /// Returns current **local-space** bounding box.
    #[inline]
    pub fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::from_radius(self.size)
    }

    /// Returns current **world-space** bounding box.
    pub fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    pub(crate) fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.set_texture(resource_manager.map_texture(self.texture.clone()));
    }

    // Prefab inheritance resolving.
    pub(crate) fn inherit(&mut self, parent: &Node) {
        self.base.inherit_properties(parent);

        // TODO: Add properties. https://github.com/FyroxEngine/Fyrox/issues/282
    }
}

impl Visit for Sprite {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.texture.visit("Texture", visitor)?;
        self.color.visit("Color", visitor)?;
        self.size.visit("Size", visitor)?;
        self.rotation.visit("Rotation", visitor)?;
        self.base.visit("Base", visitor)?;

        visitor.leave_region()
    }
}

/// Sprite builder allows you to construct sprite in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct SpriteBuilder {
    base_builder: BaseBuilder,
    texture: Option<Texture>,
    color: Color,
    size: f32,
    rotation: f32,
}

impl SpriteBuilder {
    /// Creates new builder with default state (white opaque color, 0.2 size, zero rotation).
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            texture: None,
            color: Color::WHITE,
            size: 0.2,
            rotation: 0.0,
        }
    }

    /// Sets desired texture.
    pub fn with_texture(mut self, texture: Texture) -> Self {
        self.texture = Some(texture);
        self
    }

    /// Sets desired texture.
    pub fn with_opt_texture(mut self, texture: Option<Texture>) -> Self {
        self.texture = texture;
        self
    }

    /// Sets desired color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets desired size.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets desired rotation.
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    fn build_sprite(self) -> Sprite {
        Sprite {
            base: self.base_builder.build_base(),
            texture: self.texture,
            color: self.color,
            size: self.size,
            rotation: self.rotation,
        }
    }

    /// Creates new sprite instance.
    pub fn build_node(self) -> Node {
        Node::Sprite(self.build_sprite())
    }

    /// Creates new sprite instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
