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

//! Rectangle is the simplest "2D" node, it can be used to create "2D" graphics. 2D is in quotes
//! here because the node is actually a 3D node, like everything else in the engine.
//!
//! See [`Rectangle`] docs for more info.

use crate::scene::animation::spritesheet::SpriteSheetAnimation;
use crate::{
    core::{
        algebra::{Point3, Vector2, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, Rect, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        value_as_u8_slice,
        variable::InheritableVariable,
        visitor::prelude::*,
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
use std::{
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};

/// A vertex for static meshes.
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)] // OpenGL expects this structure packed as in C
pub struct RectangleVertex {
    /// Position of vertex in local coordinates.
    pub position: Vector3<f32>,
    /// Texture coordinates.
    pub tex_coord: Vector2<f32>,
    /// Diffuse color.
    pub color: Color,
}

impl VertexTrait for RectangleVertex {
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
                usage: VertexAttributeUsage::Color,
                data_type: VertexAttributeDataType::U8,
                size: 4,
                divisor: 0,
                shader_location: 2,
                normalized: true,
            },
        ]
    }
}

impl PartialEq for RectangleVertex {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && self.tex_coord == other.tex_coord
            && self.color == other.color
    }
}

// This is safe because Vertex is tightly packed struct with C representation
// there is no padding bytes which may contain garbage data. This is strictly
// required because vertices will be directly passed on GPU.
impl Hash for RectangleVertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        #[allow(unsafe_code)]
        unsafe {
            let bytes = self as *const Self as *const u8;
            state.write(std::slice::from_raw_parts(
                bytes,
                std::mem::size_of::<Self>(),
            ))
        }
    }
}

/// Rectangle is the simplest "2D" node, it can be used to create "2D" graphics. 2D is in quotes
/// here because the node is actually a 3D node, like everything else in the engine.
///
/// # Flipping
///
/// It is possible to flip the sprite on both axes, vertical and horizontal. Use [`Rectangle::set_flip_x`]
/// and [`Rectangle::set_flip_y`] methods to flip the sprite on desired axes.
///
/// ## Material
///
/// Rectangles could use an arbitrary material for rendering, which means that you have full control
/// on how the rectangle will be rendered on screen.
///
/// By default, the rectangle uses standard 2D material which has only one property - `diffuseTexture`.
/// You could use it to set a texture for your rectangle:
///
/// ```rust
/// # use fyrox_impl::{
/// #     core::sstorage::ImmutableString,
/// #     material::{shader::SamplerFallback, MaterialProperty},
/// #     resource::texture::TextureResource,
/// #     scene::dim2::rectangle::Rectangle,
/// # };
/// #
/// fn set_texture(rect: &mut Rectangle, texture: Option<TextureResource>) {
///     rect.material()
///         .data_ref()
///         .bind("diffuseTexture", texture);
/// }
/// ```
///
/// The same property could also be changed in the editor using the Material Editor invoked from
/// the `Material` property in the Inspector.
///
/// ## Performance
///
/// Rectangles use batching to let you draw tons of rectangles with high performance.
///
/// ## Specifying region for rendering
///
/// You can specify a portion of the texture that will be used for rendering using [`Self::set_uv_rect`]
/// method. This is especially useful if you need to create sprite sheet animation, you use the single
/// image, but just changing portion for rendering. Keep in mind that the coordinates are normalized
/// which means `[0; 0]` corresponds to top-left corner of the texture and `[1; 1]` corresponds to
/// right-bottom corner.
#[derive(Reflect, Debug, Clone, Visit, ComponentProvider)]
#[reflect(derived_type = "Node")]
pub struct Rectangle {
    base: Base,

    #[reflect(setter = "set_color")]
    color: InheritableVariable<Color>,

    #[reflect(setter = "set_uv_rect")]
    uv_rect: InheritableVariable<Rect<f32>>,

    material: InheritableVariable<MaterialResource>,

    #[reflect(setter = "set_flip_x")]
    flip_x: InheritableVariable<bool>,

    #[reflect(setter = "set_flip_y")]
    flip_y: InheritableVariable<bool>,
}

impl Default for Rectangle {
    fn default() -> Self {
        Self {
            base: Default::default(),
            color: Default::default(),
            uv_rect: InheritableVariable::new_modified(Rect::new(0.0, 0.0, 1.0, 1.0)),
            material: InheritableVariable::new_modified(MaterialResource::new_ok(
                Uuid::new_v4(),
                Default::default(),
                Material::standard_2d(),
            )),
            flip_x: Default::default(),
            flip_y: Default::default(),
        }
    }
}

impl Deref for Rectangle {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Rectangle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl TypeUuidProvider for Rectangle {
    fn type_uuid() -> Uuid {
        uuid!("bb57b5e0-367a-4490-bf30-7f547407d5b5")
    }
}

impl Rectangle {
    /// Returns current color of the rectangle.
    pub fn color(&self) -> Color {
        *self.color
    }

    /// Returns a reference to the current material used by the rectangle.
    pub fn material(&self) -> &InheritableVariable<MaterialResource> {
        &self.material
    }

    /// Returns a reference to the current material used by the rectangle.
    pub fn material_mut(&mut self) -> &mut InheritableVariable<MaterialResource> {
        &mut self.material
    }

    /// Sets color of the rectangle.
    pub fn set_color(&mut self, color: Color) -> Color {
        self.color.set_value_and_mark_modified(color)
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

    /// Enables (`true`) or disables (`false`) horizontal flipping of the rectangle.
    pub fn set_flip_x(&mut self, flip: bool) -> bool {
        self.flip_x.set_value_and_mark_modified(flip)
    }

    /// Returns `true` if the rectangle is flipped horizontally, `false` - otherwise.
    pub fn is_flip_x(&self) -> bool {
        *self.flip_x
    }

    /// Enables (`true`) or disables (`false`) vertical flipping of the rectangle.
    pub fn set_flip_y(&mut self, flip: bool) -> bool {
        self.flip_y.set_value_and_mark_modified(flip)
    }

    /// Returns `true` if the rectangle is flipped vertically, `false` - otherwise.
    pub fn is_flip_y(&self) -> bool {
        *self.flip_y
    }

    /// Applies the given sprite sheet animation. This method assumes that the rectangle's material
    /// has the `diffuseTexture` resource.
    pub fn apply_animation(&mut self, animation: &SpriteSheetAnimation) {
        self.material()
            .data_ref()
            .bind("diffuseTexture", animation.texture());
        self.set_uv_rect(animation.current_frame_uv_rect().unwrap_or_default());
    }
}

impl ConstructorProvider<Node, Graph> for Rectangle {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_variant("Rectangle (2D Sprite)", |_| {
                RectangleBuilder::new(BaseBuilder::new().with_name("Sprite (2D)"))
                    .build_node()
                    .into()
            })
            .with_group("2D")
    }
}

impl NodeTrait for Rectangle {
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::unit()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.should_be_rendered(ctx.frustum, ctx.render_mask) {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) {
            return RdcControlFlow::Continue;
        }

        let global_transform = self.global_transform();

        type Vertex = RectangleVertex;

        let lx = self.uv_rect.position.x;
        let rx = self.uv_rect.position.x + self.uv_rect.size.x;
        let ty = self.uv_rect.position.y;
        let by = self.uv_rect.position.y + self.uv_rect.size.y;

        let vertices = [
            Vertex {
                position: global_transform
                    .transform_point(&Point3::new(0.5, 0.5, 0.0))
                    .coords,
                tex_coord: Vector2::new(
                    if *self.flip_x { rx } else { lx },
                    if *self.flip_y { by } else { ty },
                ),
                color: *self.color,
            },
            Vertex {
                position: global_transform
                    .transform_point(&Point3::new(-0.5, 0.5, 0.0))
                    .coords,
                tex_coord: Vector2::new(
                    if *self.flip_x { lx } else { rx },
                    if *self.flip_y { by } else { ty },
                ),
                color: *self.color,
            },
            Vertex {
                position: global_transform
                    .transform_point(&Point3::new(-0.5, -0.5, 0.0))
                    .coords,
                tex_coord: Vector2::new(
                    if *self.flip_x { lx } else { rx },
                    if *self.flip_y { ty } else { by },
                ),
                color: *self.color,
            },
            Vertex {
                position: global_transform
                    .transform_point(&Point3::new(0.5, -0.5, 0.0))
                    .coords,
                tex_coord: Vector2::new(
                    if *self.flip_x { rx } else { lx },
                    if *self.flip_y { ty } else { by },
                ),
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

/// Allows you to create rectangle in declarative manner.
pub struct RectangleBuilder {
    base_builder: BaseBuilder,
    color: Color,
    uv_rect: Rect<f32>,
    material: MaterialResource,
    flip_x: bool,
    flip_y: bool,
}

impl RectangleBuilder {
    /// Creates new rectangle builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            color: Color::WHITE,
            uv_rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            material: MaterialResource::new_ok(
                Uuid::new_v4(),
                Default::default(),
                Material::standard_2d(),
            ),
            flip_x: false,
            flip_y: false,
        }
    }

    /// Sets desired color of the rectangle.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets desired portion of the texture for the rectangle. See [`Rectangle::set_uv_rect`]
    /// for more info.
    pub fn with_uv_rect(mut self, uv_rect: Rect<f32>) -> Self {
        self.uv_rect = uv_rect;
        self
    }

    /// Sets the desired material of the rectangle.
    pub fn with_material(mut self, material: MaterialResource) -> Self {
        self.material = material;
        self
    }

    /// Flips the rectangle horizontally.
    pub fn with_flip_x(mut self, flip_x: bool) -> Self {
        self.flip_x = flip_x;
        self
    }

    /// Flips the rectangle vertically.
    pub fn with_flip_y(mut self, flip_y: bool) -> Self {
        self.flip_y = flip_y;
        self
    }

    /// Creates new [`Rectangle`] instance.
    pub fn build_rectangle(self) -> Rectangle {
        Rectangle {
            base: self.base_builder.build_base(),
            color: self.color.into(),
            uv_rect: self.uv_rect.into(),
            material: self.material.into(),
            flip_x: self.flip_x.into(),
            flip_y: self.flip_y.into(),
        }
    }

    /// Creates new [`Rectangle`] instance.
    pub fn build_node(self) -> Node {
        Node::new(self.build_rectangle())
    }

    /// Creates new [`Rectangle`] instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Rectangle> {
        graph.add_node(self.build_node()).to_variant()
    }
}
