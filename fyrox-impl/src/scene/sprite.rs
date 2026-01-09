// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Contains all structures and methods to create and manage sprites.
//!
//! For more info see [`Sprite`].

use crate::{
    core::{
        algebra::{Vector2, Vector3, Vector4},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, Rect, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        value_as_u8_slice,
        variable::InheritableVariable,
        visitor::{Visit, VisitResult, Visitor},
    },
    graph::{constructor::ConstructorProvider, SceneGraph},
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
        node::{constructor::NodeConstructor, Node, NodeTrait, RdcControlFlow},
    },
};
use bytemuck::{Pod, Zeroable};
use std::ops::{Deref, DerefMut};

/// A vertex for sprites.
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct SpriteVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates.
    pub tex_coord: Vector2<f32>,
    /// Sprite parameters: x - size, y - rotation, z - dx, w - dy.
    pub params: Vector4<f32>,
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
                size: 4,
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

/// Sprite is a billboard which always faces towards the camera. It can be used as a "model" for bullets,
/// and so on.
///
/// # Performance
///
/// Sprites rendering uses batching to reduce the number of draw calls - it basically merges multiple
/// sprites with the same material into one mesh and renders it in a single draw call which is quite
/// fast and can handle tens of thousands of sprites with ease. You should not, however, use sprites to
/// make particle systems, use [ParticleSystem](super::particle_system::ParticleSystem) instead.
///
/// # Flipping
///
/// It is possible to flip the sprite on both axes, vertical and horizontal. Use [`Sprite::set_flip_x`]
/// and [`Sprite::set_flip_y`] methods to flip the sprite on desired axes.
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
///         .bind("smoke.png", resource_manager.request::<Texture>("smoke.png"));
///
///     SpriteBuilder::new(BaseBuilder::new())
///         .with_material(MaterialResource::new_embedded(material))
///         .build(graph)
/// }
/// ```
///
/// Keep in mind, that this example creates new material instance each call of the method and
/// **does not** reuse it. Ideally, you should reuse the shared material across multiple instances
/// to get the best possible performance. Otherwise, each your sprite will be put in a separate batch
/// which will force your GPU to render a single sprite in dedicated draw call which is quite slow.
#[derive(Debug, Reflect, Clone, ComponentProvider, Visit)]
#[reflect(derived_type = "Node")]
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

    #[reflect(setter = "set_flip_x")]
    flip_x: InheritableVariable<bool>,

    #[reflect(setter = "set_flip_y")]
    flip_y: InheritableVariable<bool>,
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

    /// Enables (`true`) or disables (`false`) horizontal flipping of the sprite.
    pub fn set_flip_x(&mut self, flip: bool) -> bool {
        self.flip_x.set_value_and_mark_modified(flip)
    }

    /// Returns `true` if the sprite is flipped horizontally, `false` - otherwise.
    pub fn is_flip_x(&self) -> bool {
        *self.flip_x
    }

    /// Enables (`true`) or disables (`false`) vertical flipping of the sprite.
    pub fn set_flip_y(&mut self, flip: bool) -> bool {
        self.flip_y.set_value_and_mark_modified(flip)
    }

    /// Returns `true` if the sprite is flipped vertically, `false` - otherwise.
    pub fn is_flip_y(&self) -> bool {
        *self.flip_y
    }
}

impl ConstructorProvider<Node, Graph> for Sprite {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>().with_variant("Sprite (3D)", |_| {
            SpriteBuilder::new(BaseBuilder::new().with_name("Sprite"))
                .build_node()
                .into()
        })
    }
}

impl NodeTrait for Sprite {
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
        if !self.should_be_rendered(ctx.frustum, ctx.render_mask) {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) || !self.cast_shadows() {
            return RdcControlFlow::Continue;
        }

        let position = self.global_position();

        type Vertex = SpriteVertex;

        let lx = self.uv_rect.position.x;
        let rx = self.uv_rect.position.x + self.uv_rect.size.x;
        let ty = self.uv_rect.position.y;
        let by = self.uv_rect.position.y + self.uv_rect.size.y;

        let vertices = [
            Vertex {
                position,
                tex_coord: Vector2::new(
                    if *self.flip_x { lx } else { rx },
                    if *self.flip_y { by } else { ty },
                ),
                params: Vector4::new(*self.size, *self.rotation, 0.5, 0.5),
                color: *self.color,
            },
            Vertex {
                position,
                tex_coord: Vector2::new(
                    if *self.flip_x { rx } else { lx },
                    if *self.flip_y { by } else { ty },
                ),
                params: Vector4::new(*self.size, *self.rotation, -0.5, 0.5),
                color: *self.color,
            },
            Vertex {
                position,
                tex_coord: Vector2::new(
                    if *self.flip_x { rx } else { lx },
                    if *self.flip_y { ty } else { by },
                ),
                params: Vector4::new(*self.size, *self.rotation, -0.5, -0.5),
                color: *self.color,
            },
            Vertex {
                position,
                tex_coord: Vector2::new(
                    if *self.flip_x { lx } else { rx },
                    if *self.flip_y { ty } else { by },
                ),
                params: Vector4::new(*self.size, *self.rotation, 0.5, -0.5),
                color: *self.color,
            },
        ];

        let triangles = [TriangleDefinition([0, 1, 2]), TriangleDefinition([0, 2, 3])];

        let sort_index = ctx.calculate_sorting_index(self.global_position());

        ctx.storage.push_triangles(
            ctx.dynamic_surface_cache,
            Vertex::layout(),
            &self.material,
            RenderPath::Forward,
            sort_index,
            self.handle(),
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
    flip_x: bool,
    flip_y: bool,
}

impl SpriteBuilder {
    /// Creates new builder with default state (white opaque color, 0.2 size, zero rotation).
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            material: MaterialResource::new_ok(
                Uuid::new_v4(),
                Default::default(),
                Material::standard_sprite(),
            ),
            uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            color: Color::WHITE,
            size: 0.2,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
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

    /// Flips the sprite horizontally.
    pub fn with_flip_x(mut self, flip_x: bool) -> Self {
        self.flip_x = flip_x;
        self
    }

    /// Flips the sprite vertically.
    pub fn with_flip_y(mut self, flip_y: bool) -> Self {
        self.flip_y = flip_y;
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
            flip_x: self.flip_x.into(),
            flip_y: self.flip_y.into(),
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
