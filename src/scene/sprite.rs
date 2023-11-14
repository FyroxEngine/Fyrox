//! Contains all structures and methods to create and manage sprites.
//!
//! For more info see [`Sprite`].

use crate::{
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        log::Log,
        math::{aabb::AxisAlignedBoundingBox, Rect, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        sstorage::ImmutableString,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::{Visit, VisitResult, Visitor},
        TypeUuidProvider,
    },
    material::{shader::SamplerFallback, Material, PropertyValue, SharedMaterial},
    renderer::{self, batch::RenderContext},
    resource::texture::TextureResource,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::{
            buffer::{
                VertexAttributeDataType, VertexAttributeDescriptor, VertexAttributeUsage,
                VertexTrait,
            },
            RenderPath,
        },
        node::{Node, NodeTrait},
    },
};
use std::ops::{Deref, DerefMut};

/// A vertex for sprites.
#[derive(Copy, Clone, Debug, Default)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct SpriteVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates.
    pub tex_coord: Vector2<f32>,
    /// Sprite parameters: x - size, y - rotation.
    pub params: Vector2<f32>,
    /// Diffuse color.
    pub color: Color,
}

impl VertexTrait for SpriteVertex {
    fn layout() -> &'static [VertexAttributeDescriptor] {
        &[
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Position,
                data_type: VertexAttributeDataType::F32,
                size: 3,
                divisor: 0,
                shader_location: 0,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::TexCoord0,
                data_type: VertexAttributeDataType::F32,
                size: 2,
                divisor: 0,
                shader_location: 1,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Custom0,
                data_type: VertexAttributeDataType::F32,
                size: 2,
                divisor: 0,
                shader_location: 2,
                normalized: false,
            },
            VertexAttributeDescriptor {
                usage: VertexAttributeUsage::Color,
                data_type: VertexAttributeDataType::U8,
                size: 4,
                divisor: 0,
                shader_location: 3,
                normalized: true,
            },
        ]
    }
}

/// Sprite is a billboard which always faces towards camera. It can be used as a "model" for bullets, and so on.
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
///     asset::manager::ResourceManager,
///     core::pool::{Handle},
/// };
/// use fyrox::resource::texture::Texture;
///
/// fn create_smoke(resource_manager: ResourceManager, graph: &mut Graph) -> Handle<Node> {
///     SpriteBuilder::new(BaseBuilder::new())
///         .with_texture(resource_manager.request::<Texture, _>("smoke.png"))
///         .build(graph)
/// }
/// ```
#[derive(Debug, Reflect, Clone)]
pub struct Sprite {
    base: Base,

    #[reflect(setter = "set_uv_rect")]
    uv_rect: InheritableVariable<Rect<f32>>,

    material: InheritableVariable<SharedMaterial>,

    #[reflect(setter = "set_color")]
    color: InheritableVariable<Color>,

    #[reflect(min_value = 0.0, step = 0.1)]
    #[reflect(setter = "set_size")]
    size: InheritableVariable<f32>,

    #[reflect(setter = "set_rotation")]
    rotation: InheritableVariable<f32>,
}

impl Visit for Sprite {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        if region.is_reading() {
            let mut texture: InheritableVariable<Option<TextureResource>> = Default::default();
            if texture.visit("Texture", &mut region).is_ok() {
                // Backward compatibility.
                let mut material = Material::standard_sprite();
                Log::verify(material.set_property(
                    &ImmutableString::new("diffuseTexture"),
                    PropertyValue::Sampler {
                        value: (*texture).clone(),
                        fallback: SamplerFallback::White,
                    },
                ));
                self.material = SharedMaterial::new(material).into();
            } else {
                self.material.visit("Material", &mut region)?;
            }
        } else {
            self.material.visit("Material", &mut region)?;
        }

        self.base.visit("Base", &mut region)?;
        self.color.visit("Color", &mut region)?;
        self.size.visit("Size", &mut region)?;
        self.rotation.visit("Rotation", &mut region)?;

        // Backward compatibility.
        let _ = self.uv_rect.visit("Material", &mut region);

        Ok(())
    }
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
        self.size.set_value_and_mark_modified(size)
    }

    /// Returns current size of sprite.
    pub fn size(&self) -> f32 {
        *self.size
    }

    /// Sets new color of sprite. Default is White.
    pub fn set_color(&mut self, color: Color) -> Color {
        self.color.set_value_and_mark_modified(color)
    }

    /// Returns current color of sprite.
    pub fn color(&self) -> Color {
        *self.color
    }

    /// Sets rotation around "look" axis in radians. Default is 0.0.
    pub fn set_rotation(&mut self, rotation: f32) -> f32 {
        self.rotation.set_value_and_mark_modified(rotation)
    }

    /// Returns rotation in radians.
    pub fn rotation(&self) -> f32 {
        *self.rotation
    }

    /// Returns a reference to the current material used by the rectangle.
    pub fn material(&self) -> &InheritableVariable<SharedMaterial> {
        &self.material
    }

    /// Returns a reference to the current material used by the rectangle.
    pub fn material_mut(&mut self) -> &mut InheritableVariable<SharedMaterial> {
        &mut self.material
    }

    /// Returns a rectangle that defines the region in texture which will be rendered. The coordinates are normalized
    /// which means `[0; 0]` corresponds to top-left corner of the texture and `[1; 1]` corresponds to right-bottom
    /// corner.
    pub fn uv_rect(&self) -> Rect<f32> {
        *self.uv_rect
    }

    /// Sets a rectangle that defines the region in texture which will be rendered. The coordinates are normalized
    /// which means `[0; 0]` corresponds to top-left corner of the texture and `[1; 1]` corresponds to right-bottom
    /// corner.
    ///
    /// The coordinates can exceed `[1; 1]` boundary to create tiling effect (keep in mind that tiling should be
    /// enabled in texture options).
    ///
    /// The default value is `(0, 0, 1, 1)` rectangle which corresponds to entire texture.
    pub fn set_uv_rect(&mut self, uv_rect: Rect<f32>) -> Rect<f32> {
        self.uv_rect.set_value_and_mark_modified(uv_rect)
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

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) {
        if !self.global_visibility()
            || !self.is_globally_enabled()
            || !ctx.frustum.is_intersects_aabb(&self.world_bounding_box())
        {
            return;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) || !self.cast_shadows() {
            return;
        }

        let position = self.global_position();
        let params = Vector2::new(*self.size, *self.rotation);

        let vertices = [
            SpriteVertex {
                position,
                tex_coord: self.uv_rect.right_top_corner(),
                params,
                color: *self.color,
            },
            SpriteVertex {
                position,
                tex_coord: self.uv_rect.left_top_corner(),
                params,
                color: *self.color,
            },
            SpriteVertex {
                position,
                tex_coord: self.uv_rect.left_bottom_corner(),
                params,
                color: *self.color,
            },
            SpriteVertex {
                position,
                tex_coord: self.uv_rect.right_bottom_corner(),
                params,
                color: *self.color,
            },
        ];

        let triangles = [TriangleDefinition([0, 1, 2]), TriangleDefinition([2, 3, 0])];

        ctx.storage.push_triangles(
            vertices.into_iter(),
            triangles.into_iter(),
            &self.material,
            RenderPath::Forward,
            0,
            0,
            false,
            self.self_handle,
        )
    }
}

/// Sprite builder allows you to construct sprite in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct SpriteBuilder {
    base_builder: BaseBuilder,
    uv_rect: Rect<f32>,
    material: SharedMaterial,
    color: Color,
    size: f32,
    rotation: f32,
}

impl SpriteBuilder {
    /// Creates new builder with default state (white opaque color, 0.2 size, zero rotation).
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            material: SharedMaterial::new(Material::standard_sprite()),
            uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            color: Color::WHITE,
            size: 0.2,
            rotation: 0.0,
        }
    }

    /// Sets desired portion of the texture for the rectangle. See [`Rectangle::set_uv_rect`]
    /// for more info.
    pub fn with_uv_rect(mut self, uv_rect: Rect<f32>) -> Self {
        self.uv_rect = uv_rect;
        self
    }

    /// Sets the desired material of the rectangle.
    pub fn with_material(mut self, material: SharedMaterial) -> Self {
        self.material = material;
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
            material: self.material.into(),
            uv_rect: self.uv_rect.into(),
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
