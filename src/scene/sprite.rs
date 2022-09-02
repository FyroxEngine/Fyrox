//! Contains all structures and methods to create and manage sprites.
//!
//! For more info see [`Sprite`].

use crate::{
    core::{
        color::Color,
        inspect::{Inspect, PropertyInfo},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::{Visit, VisitResult, Visitor},
    },
    engine::resource_manager::ResourceManager,
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::{map::NodeHandleMap, Graph},
        node::{Node, NodeTrait, TypeUuidProvider},
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
#[derive(Debug, Inspect, Reflect, Clone, Visit)]
pub struct Sprite {
    base: Base,

    #[reflect(setter = "set_texture")]
    texture: InheritableVariable<Option<Texture>>,

    #[reflect(setter = "set_color")]
    color: InheritableVariable<Color>,

    #[inspect(min_value = 0.0, step = 0.1)]
    #[reflect(setter = "set_size")]
    size: InheritableVariable<f32>,

    #[reflect(setter = "set_rotation")]
    rotation: InheritableVariable<f32>,
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

impl TypeUuidProvider for Sprite {
    fn type_uuid() -> Uuid {
        uuid!("60fd7e34-46c1-4ae9-8803-1f5f4c341518")
    }
}

impl Sprite {
    /// Sets new size of sprite. Since sprite is always square, size defines half of width or height, so actual size
    /// will be doubled. Default value is 0.2.    
    ///
    /// Negative values could be used to "inverse" the image on the sprite.
    pub fn set_size(&mut self, size: f32) -> f32 {
        self.size.set(size)
    }

    /// Returns current size of sprite.
    pub fn size(&self) -> f32 {
        *self.size
    }

    /// Sets new color of sprite. Default is White.
    pub fn set_color(&mut self, color: Color) -> Color {
        self.color.set(color)
    }

    /// Returns current color of sprite.
    pub fn color(&self) -> Color {
        *self.color
    }

    /// Sets rotation around "look" axis in radians. Default is 0.0.
    pub fn set_rotation(&mut self, rotation: f32) -> f32 {
        self.rotation.set(rotation)
    }

    /// Returns rotation in radians.
    pub fn rotation(&self) -> f32 {
        *self.rotation
    }

    /// Sets new texture for sprite. Default is None.
    pub fn set_texture(&mut self, texture: Option<Texture>) -> Option<Texture> {
        self.texture.set(texture)
    }

    /// Returns current texture of sprite. Can be None if sprite has no texture.
    pub fn texture(&self) -> Option<Texture> {
        (*self.texture).clone()
    }

    /// Returns current texture of sprite by ref. Can be None if sprite has no texture.
    pub fn texture_ref(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }
}

impl NodeTrait for Sprite {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::from_radius(*self.size)
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base.restore_resources(resource_manager.clone());

        let mut state = resource_manager.state();
        let texture_container = &mut state.containers_mut().textures;
        texture_container.try_restore_inheritable_resource(&mut self.texture);
    }

    fn remap_handles(&mut self, old_new_mapping: &NodeHandleMap) {
        self.base.remap_handles(old_new_mapping);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
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
            texture: self.texture.into(),
            color: self.color.into(),
            size: self.size.into(),
            rotation: self.rotation.into(),
        }
    }

    /// Creates new sprite instance.
    pub fn build_node(self) -> Node {
        Node::new(self.build_sprite())
    }

    /// Creates new sprite instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::core::reflect::Reflect;
    use crate::core::variable::try_inherit_properties;
    use crate::{
        core::color::Color,
        resource::texture::test::create_test_texture,
        scene::{
            base::{test::check_inheritable_properties_equality, BaseBuilder},
            sprite::{Sprite, SpriteBuilder},
        },
    };

    #[test]
    fn test_sprite_inheritance() {
        let parent = SpriteBuilder::new(BaseBuilder::new())
            .with_color(Color::opaque(1, 2, 3))
            .with_rotation(1.0)
            .with_size(2.0)
            .with_texture(create_test_texture())
            .build_node();

        let mut child = SpriteBuilder::new(BaseBuilder::new()).build_sprite();

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        let parent = parent.cast::<Sprite>().unwrap();

        check_inheritable_properties_equality(&child.base, &parent.base);
        check_inheritable_properties_equality(&child, parent)
    }
}
