//! Contains all structures and methods to create and manage sprites.
//!
//! For more info see [`Sprite`].

use crate::scene::node::RdcControlFlow;
use crate::{
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, Rect, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::{Visit, VisitResult, Visitor},
        TypeUuidProvider,
    },
    material,
    material::{Material, MaterialResource},
    renderer::{self, bundle::RenderContext},
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
use bytemuck::{Pod, Zeroable};
use fyrox_core::value_as_u8_slice;
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

/// A vertex for sprites.
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
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

/// Sprite is a billboard which always faces towards camera. It can be used as a "model" for bullets,
/// and so on.
///
/// # Depth sorting
///
/// Sprites are **not** depth-sorted so there could be some blending issues if multiple sprites are
/// stacked one behind another.
///
/// # Performance
///
/// Sprites rendering uses batching to reduce amount of draw calls - it basically merges multiple
/// sprites with the same material into one mesh and renders it in a single draw call which is quite
/// fast and can handle tens of thousands sprites with ease. You should not, however, use sprites to
/// make particle systems, use [ParticleSystem](super::particle_system::ParticleSystem) instead.
///
/// # Example
///
/// The following example creates a new sprite node with a material, that uses a simple smoke
/// texture:
///
/// ```rust
/// # use fyrox_impl::{
/// #     asset::manager::ResourceManager,
/// #     core::pool::Handle,
/// #     material::{Material, MaterialResource},
/// #     resource::texture::Texture,
/// #     scene::{base::BaseBuilder, graph::Graph, node::Node, sprite::SpriteBuilder},
/// # };
/// #
/// fn create_smoke(resource_manager: ResourceManager, graph: &mut Graph) -> Handle<Node> {
///     let mut material = Material::standard_sprite();
///
///     material
///         .set_texture(
///             &"smoke.png".into(),
///             Some(resource_manager.request::<Texture>("smoke.png")),
///         )
///         .unwrap();
///
///     SpriteBuilder::new(BaseBuilder::new())
///         .with_material(MaterialResource::new_ok(Default::default(), material))
///         .build(graph)
/// }
/// ```
///
/// Keep in mind, that this example creates new material instance each call of the method and
/// **does not** reuse it. Ideally, you should reuse the shared material across multiple instances
/// to get best possible performance. Otherwise, each your sprite will be put in a separate batch
/// which will force your GPU to render a single sprite in dedicated draw call which is quite slow.
#[derive(Debug, Reflect, Clone)]
pub struct Sprite {
    base: Base,

    #[reflect(setter = "set_uv_rect")]
    uv_rect: InheritableVariable<Rect<f32>>,

    material: InheritableVariable<MaterialResource>,

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
            if let Some(material) =
                material::visit_old_texture_as_material(&mut region, Material::standard_sprite)
            {
                self.material = material.into();
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
        let _ = self.uv_rect.visit("UvRect", &mut region);

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

    /// Returns a reference to the current material used by the sprite.
    pub fn material(&self) -> &InheritableVariable<MaterialResource> {
        &self.material
    }

    /// Returns a reference to the current material used by the sprite.
    pub fn material_mut(&mut self) -> &mut InheritableVariable<MaterialResource> {
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

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.global_visibility()
            || !self.is_globally_enabled()
            || (self.frustum_culling()
                && !ctx
                    .frustum
                    .map_or(true, |f| f.is_intersects_aabb(&self.world_bounding_box())))
        {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) || !self.cast_shadows() {
            return RdcControlFlow::Continue;
        }

        let position = self.global_position();
        let params = Vector2::new(*self.size, *self.rotation);

        type Vertex = SpriteVertex;

        let vertices = [
            Vertex {
                position,
                tex_coord: self.uv_rect.right_top_corner(),
                params,
                color: *self.color,
            },
            Vertex {
                position,
                tex_coord: self.uv_rect.left_top_corner(),
                params,
                color: *self.color,
            },
            Vertex {
                position,
                tex_coord: self.uv_rect.left_bottom_corner(),
                params,
                color: *self.color,
            },
            Vertex {
                position,
                tex_coord: self.uv_rect.right_bottom_corner(),
                params,
                color: *self.color,
            },
        ];

        let triangles = [TriangleDefinition([0, 1, 2]), TriangleDefinition([2, 3, 0])];

        let sort_index = ctx.calculate_sorting_index(self.global_position());

        ctx.storage.push_triangles(
            Vertex::layout(),
            &self.material,
            RenderPath::Forward,
            0,
            sort_index,
            false,
            self.self_handle,
            &mut move |mut vertex_buffer, mut triangle_buffer| {
                let start_vertex_index = vertex_buffer.vertex_count();

                for vertex in vertices.iter() {
                    vertex_buffer
                        .push_vertex_raw(value_as_u8_slice(vertex))
                        .unwrap();
                }

                triangle_buffer
                    .push_triangles_iter_with_offset(start_vertex_index, triangles.into_iter());
            },
        );

        RdcControlFlow::Continue
    }
}

/// Sprite builder allows you to construct sprite in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct SpriteBuilder {
    base_builder: BaseBuilder,
    uv_rect: Rect<f32>,
    material: MaterialResource,
    color: Color,
    size: f32,
    rotation: f32,
}

impl SpriteBuilder {
    /// Creates new builder with default state (white opaque color, 0.2 size, zero rotation).
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            material: MaterialResource::new_ok(Default::default(), Material::standard_sprite()),
            uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            color: Color::WHITE,
            size: 0.2,
            rotation: 0.0,
        }
    }

    /// Sets desired portion of the texture for the sprite. See [`Sprite::set_uv_rect`]
    /// for more info.
    pub fn with_uv_rect(mut self, uv_rect: Rect<f32>) -> Self {
        self.uv_rect = uv_rect;
        self
    }

    /// Sets the desired material of the sprite.
    pub fn with_material(mut self, material: MaterialResource) -> Self {
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
