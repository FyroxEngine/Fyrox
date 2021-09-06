//! Decal is an image that gets projected to a geometry of a scene.
//!
//! For more info see [`Decal`]

use crate::{
    core::{color::Color, pool::Handle, visitor::prelude::*},
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

/// Decal is an image that gets projected to a geometry of a scene. Blood splatters, bullet holes, scratches
/// etc. are done via decals.
///
/// # Size and transformations
///
/// A decal defines a cube that projects a texture on every pixel of a scene that got into the cube. Exact cube
/// size is defines by decal's `local scale`. For example, if you have a decal with scale (1.0, 2.0, 0.1) then
/// the size of the cube (in local coordinates) will be `width = 1.0`, `height = 2.0`, `depth = 0.1`. The decal
/// can be rotated as any other scene node. Its final size and orientation is defined by the chain of
/// transformations of parent nodes.
///
/// # Masking
///
/// Often you need to ensure that decal will be applied only on desired surfaces. For example a crack on the wall
/// should not affect any surrounding objects, this can be achieved by using decal mask. Each decal has layer index,
/// it will be drawn only if the index matches the index of the object that inside of decal bounds.
///
/// # Supported maps
///
/// Currently, only diffuse and normal maps are supported. Diffuse and normal maps will be automatically projected
/// on the data stored in G-Buffer.
///
/// # Limitations
///
/// Current implementation works only with Deferred render path. Custom materials that uses Forward pass should
/// implement decals manually.
///
/// # Performance
///
/// It should be noted that decals are not cheap, keep amount (and size) of decals at reasonable values! This
/// means that unused decals (bullet holes for example) must be removed after some time.
///
/// # Example
///
/// ```
/// use rg3d::{
///         engine::resource_manager::ResourceManager,
///         core::pool::Handle,
///         scene::{
///         node::Node,
///         graph::Graph,
///         decal::DecalBuilder,
///         base::BaseBuilder,
///         transform::TransformBuilder
///     },
///     core::algebra::Vector3
/// };
///
/// fn create_bullet_hole(resource_manager: ResourceManager, graph: &mut Graph) -> Handle<Node> {
///     DecalBuilder::new(
///             BaseBuilder::new()
///                 .with_local_transform(
///                     TransformBuilder::new()
///                         .with_local_scale(Vector3::new(2.0, 2.0, 2.0))
///                         .build()
///         ))
///         .with_diffuse_texture(resource_manager.request_texture("bullet_hole.png", None))
///         .build(graph)
/// }
/// ```
#[derive(Debug, Visit, Default)]
pub struct Decal {
    base: Base,
    diffuse_texture: Option<Texture>,
    normal_texture: Option<Texture>,
    #[visit(optional)] // Backward compatibility
    color: Color,
    #[visit(optional)] // Backward compatibility
    layer: u8,
}

impl Deref for Decal {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Decal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Decal {
    /// Creates a raw copy of Decal node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            diffuse_texture: self.diffuse_texture.clone(),
            normal_texture: self.normal_texture.clone(),
            color: self.color,
            layer: self.layer,
        }
    }

    /// Sets new diffuse texture.
    pub fn set_diffuse_texture(&mut self, diffuse_texture: Option<Texture>) -> Option<Texture> {
        std::mem::replace(&mut self.diffuse_texture, diffuse_texture)
    }

    /// Returns current diffuse texture.
    pub fn diffuse_texture(&self) -> Option<&Texture> {
        self.diffuse_texture.as_ref()
    }

    /// Returns current diffuse texture.
    pub fn diffuse_texture_value(&self) -> Option<Texture> {
        self.diffuse_texture.clone()
    }

    /// Sets new normal texture.
    pub fn set_normal_texture(&mut self, normal_texture: Option<Texture>) -> Option<Texture> {
        std::mem::replace(&mut self.normal_texture, normal_texture)
    }

    /// Returns current normal texture.
    pub fn normal_texture(&self) -> Option<&Texture> {
        self.normal_texture.as_ref()
    }

    /// Returns current normal texture.
    pub fn normal_texture_value(&self) -> Option<Texture> {
        self.normal_texture.clone()
    }

    /// Sets new color for the decal.
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Returns current color of the decal.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Sets layer index of the decal. Layer index allows you to apply decals only on desired
    /// surfaces. For example, static geometry could have `index == 0` and dynamic `index == 1`.
    /// To "filter" decals all you need to do is to set appropriate layer index to decal, for
    /// example blood splatter decal will have `index == 0` in this case. In case of dynamic
    /// objects (like bots, etc.) index will be 1.
    pub fn set_layer(&mut self, layer: u8) {
        self.layer = layer;
    }

    /// Returns current layer index.
    pub fn layer(&self) -> u8 {
        self.layer
    }
}

/// Allows you to create a Decal in a declarative manner.
pub struct DecalBuilder {
    base_builder: BaseBuilder,
    diffuse_texture: Option<Texture>,
    normal_texture: Option<Texture>,
    color: Color,
    layer: u8,
}

impl DecalBuilder {
    /// Creates a new instance of the builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            diffuse_texture: None,
            normal_texture: None,
            color: Color::opaque(255, 255, 255),
            layer: 0,
        }
    }

    /// Sets desired diffuse texture.
    pub fn with_diffuse_texture(mut self, diffuse_texture: Texture) -> Self {
        self.diffuse_texture = Some(diffuse_texture);
        self
    }

    /// Sets desired normal texture.
    pub fn with_normal_texture(mut self, normal_texture: Texture) -> Self {
        self.normal_texture = Some(normal_texture);
        self
    }

    /// Sets desired decal color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets desired layer index.
    pub fn with_layer(mut self, layer: u8) -> Self {
        self.layer = layer;
        self
    }

    /// Creates new Decal node.
    pub fn build_node(self) -> Node {
        Node::Decal(Decal {
            base: self.base_builder.build_base(),
            diffuse_texture: self.diffuse_texture,
            normal_texture: self.normal_texture,
            color: self.color,
            layer: self.layer,
        })
    }

    /// Creates new instance of Decal node and puts it in the given graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
