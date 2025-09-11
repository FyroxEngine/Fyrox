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

//! See [`UiRenderer`] docs.

use crate::{
    asset::{manager::ResourceManager, untyped::ResourceKind},
    core::{
        algebra::{Matrix4, Vector2, Vector4},
        arrayvec::ArrayVec,
        color::Color,
        math::Rect,
        some_or_continue,
        sstorage::ImmutableString,
    },
    graphics::{
        buffer::BufferUsage,
        error::FrameworkError,
        framebuffer::{GpuFrameBuffer, ResourceBindGroup, ResourceBinding},
        geometry_buffer::{
            AttributeDefinition, AttributeKind, ElementsDescriptor, GpuGeometryBuffer,
            GpuGeometryBufferDescriptor, VertexBufferData, VertexBufferDescriptor,
        },
        gpu_program::ShaderResourceKind,
        server::GraphicsServer,
        uniform::StaticUniformBuffer,
        BlendFactor, BlendFunc, BlendParameters, ColorMask, CompareFunc, DrawParameters,
        ElementRange, ScissorBox, StencilFunc,
    },
    gui::{
        brush::Brush,
        draw::Command,
        draw::{CommandTexture, DrawingContext},
    },
    renderer::{
        bundle::{self, make_texture_binding},
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial, ShaderCache},
            uniform::{UniformBlockLocation, UniformBufferCache, UniformMemoryAllocator},
        },
        resources::RendererResources,
        RenderPassStatistics, TextureCache,
    },
    resource::texture::{Texture, TextureKind, TexturePixelKind, TextureResource},
};
use uuid::Uuid;

/// User interface renderer allows you to render drawing context in specified render target.
pub struct UiRenderer {
    geometry_buffer: GpuGeometryBuffer,
    clipping_geometry_buffer: GpuGeometryBuffer,
}

/// A set of parameters to render a specified user interface drawing context.
pub struct UiRenderContext<'a, 'b, 'c> {
    /// Graphics server.
    pub server: &'a dyn GraphicsServer,
    /// Viewport to where render the user interface.
    pub viewport: Rect<i32>,
    /// Frame buffer to where render the user interface.
    pub frame_buffer: &'b GpuFrameBuffer,
    /// Width of the frame buffer to where render the user interface.
    pub frame_width: f32,
    /// Height of the frame buffer to where render the user interface.
    pub frame_height: f32,
    /// Drawing context of a user interface.
    pub drawing_context: &'c DrawingContext,
    /// Renderer resources.
    pub renderer_resources: &'a RendererResources,
    /// GPU texture cache.
    pub texture_cache: &'a mut TextureCache,
    /// A reference to the cache of uniform buffers.
    pub uniform_buffer_cache: &'a mut UniformBufferCache,
    /// A reference to the render pass cache.
    pub render_pass_cache: &'a mut ShaderCache,
    /// A reference to the uniform memory allocator.
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,
    /// A reference to the resource manager.
    pub resource_manager: &'a ResourceManager,
}

fn write_uniform_blocks(
    ortho: &Matrix4<f32>,
    resolution: Vector2<f32>,
    commands: &[Command],
    uniform_memory_allocator: &mut UniformMemoryAllocator,
) -> Vec<ArrayVec<(usize, UniformBlockLocation), 8>> {
    let mut block_locations = Vec::with_capacity(commands.len());

    for cmd in commands {
        let mut command_block_locations = ArrayVec::<(usize, UniformBlockLocation), 8>::new();

        let material_data_guard = cmd.material.data_ref();
        let material = some_or_continue!(material_data_guard.as_loaded_ref());
        let shader_data_guard = material.shader().data_ref();
        let shader = some_or_continue!(shader_data_guard.as_loaded_ref());

        for resource in shader.definition.resources.iter() {
            if resource.name.as_str() == "fyrox_widgetData" {
                let mut raw_stops = [0.0; 16];
                let mut raw_colors = [Vector4::default(); 16];
                let bounds_max = cmd.bounds.right_bottom_corner();

                let (gradient_origin, gradient_end) = match cmd.brush {
                    Brush::Solid(_) => (Vector2::default(), Vector2::default()),
                    Brush::LinearGradient { from, to, .. } => (from, to),
                    Brush::RadialGradient { center, .. } => (center, Vector2::default()),
                };

                let solid_color = match cmd.brush {
                    Brush::Solid(color) => color,
                    _ => Color::WHITE,
                };
                let gradient_colors = match cmd.brush {
                    Brush::Solid(_) => &raw_colors,
                    Brush::LinearGradient { ref stops, .. }
                    | Brush::RadialGradient { ref stops, .. } => {
                        for (i, point) in stops.iter().enumerate() {
                            raw_colors[i] = point.color.as_frgba();
                        }
                        &raw_colors
                    }
                };
                let gradient_stops = match cmd.brush {
                    Brush::Solid(_) => &raw_stops,
                    Brush::LinearGradient { ref stops, .. }
                    | Brush::RadialGradient { ref stops, .. } => {
                        for (i, point) in stops.iter().enumerate() {
                            raw_stops[i] = point.stop;
                        }
                        &raw_stops
                    }
                };
                let brush_type = match cmd.brush {
                    Brush::Solid(_) => 0,
                    Brush::LinearGradient { .. } => 1,
                    Brush::RadialGradient { .. } => 2,
                };
                let gradient_point_count = match cmd.brush {
                    Brush::Solid(_) => 0,
                    Brush::LinearGradient { ref stops, .. }
                    | Brush::RadialGradient { ref stops, .. } => stops.len() as i32,
                };

                let is_font_texture = matches!(cmd.texture, CommandTexture::Font { .. });

                let buffer = StaticUniformBuffer::<2048>::new()
                    .with(ortho)
                    .with(&solid_color)
                    .with(gradient_colors.as_slice())
                    .with(gradient_stops.as_slice())
                    .with(&gradient_origin)
                    .with(&gradient_end)
                    .with(&resolution)
                    .with(&cmd.bounds.position)
                    .with(&bounds_max)
                    .with(&is_font_texture)
                    .with(&cmd.opacity)
                    .with(&brush_type)
                    .with(&gradient_point_count);

                command_block_locations
                    .push((resource.binding, uniform_memory_allocator.allocate(buffer)));
            } else if let ShaderResourceKind::PropertyGroup(ref shader_property_group) =
                resource.kind
            {
                let mut buf = StaticUniformBuffer::<16384>::new();

                if let Some(material_property_group) =
                    material.property_group_ref(resource.name.clone())
                {
                    bundle::write_with_material(
                        shader_property_group,
                        material_property_group,
                        |c, n| c.property_ref(n.clone()).map(|p| p.as_ref()),
                        &mut buf,
                    );
                } else {
                    bundle::write_shader_values(shader_property_group, &mut buf)
                }

                command_block_locations
                    .push((resource.binding, uniform_memory_allocator.allocate(buf)));
            }
        }

        block_locations.push(command_block_locations);
    }

    block_locations
}

impl UiRenderer {
    pub(in crate::renderer) fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let geometry_buffer_desc = GpuGeometryBufferDescriptor {
            name: "UiGeometryBuffer",
            elements: ElementsDescriptor::Triangles(&[]),
            buffers: &[VertexBufferDescriptor {
                usage: BufferUsage::DynamicDraw,
                attributes: &[
                    AttributeDefinition {
                        location: 0,
                        kind: AttributeKind::Float,
                        component_count: 2,
                        normalized: false,
                        divisor: 0,
                    },
                    AttributeDefinition {
                        location: 1,
                        kind: AttributeKind::Float,
                        component_count: 2,
                        normalized: false,
                        divisor: 0,
                    },
                    AttributeDefinition {
                        location: 2,
                        kind: AttributeKind::UnsignedByte,
                        component_count: 4,
                        normalized: true, // Make sure [0; 255] -> [0; 1]
                        divisor: 0,
                    },
                ],
                data: VertexBufferData::new::<crate::gui::draw::Vertex>(None),
            }],
            usage: BufferUsage::DynamicDraw,
        };

        let clipping_geometry_buffer_desc = GpuGeometryBufferDescriptor {
            name: "UiClippingGeometryBuffer",
            elements: ElementsDescriptor::Triangles(&[]),
            buffers: &[VertexBufferDescriptor {
                usage: BufferUsage::DynamicDraw,
                attributes: &[
                    // We're interested only in position. Fragment shader won't run for clipping geometry anyway.
                    AttributeDefinition {
                        location: 0,
                        kind: AttributeKind::Float,
                        component_count: 2,
                        normalized: false,
                        divisor: 0,
                    },
                ],
                data: VertexBufferData::new::<crate::gui::draw::Vertex>(None),
            }],
            usage: BufferUsage::DynamicDraw,
        };

        Ok(Self {
            geometry_buffer: server.create_geometry_buffer(geometry_buffer_desc)?,
            clipping_geometry_buffer: server
                .create_geometry_buffer(clipping_geometry_buffer_desc)?,
        })
    }

    /// Renders given UI's drawing context to specified frame buffer.
    pub fn render(
        &mut self,
        args: UiRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let UiRenderContext {
            server,
            viewport,
            frame_buffer,
            frame_width,
            frame_height,
            drawing_context,
            renderer_resources,
            texture_cache,
            uniform_buffer_cache,
            render_pass_cache,
            uniform_memory_allocator,
            resource_manager,
        } = args;

        let mut statistics = RenderPassStatistics::default();

        self.geometry_buffer
            .set_buffer_data_of_type(0, drawing_context.get_vertices());
        self.geometry_buffer
            .set_triangles(drawing_context.get_triangles());

        let ortho = Matrix4::new_orthographic(0.0, frame_width, frame_height, 0.0, -1.0, 1.0);
        let resolution = Vector2::new(frame_width, frame_height);

        let uniform_blocks = write_uniform_blocks(
            &ortho,
            resolution,
            drawing_context.get_commands(),
            uniform_memory_allocator,
        );

        uniform_memory_allocator.upload(server)?;

        for (cmd, command_uniform_blocks) in
            drawing_context.get_commands().iter().zip(uniform_blocks)
        {
            let mut clip_bounds = cmd.clip_bounds;
            clip_bounds.position.x = clip_bounds.position.x.floor();
            clip_bounds.position.y = clip_bounds.position.y.floor();
            clip_bounds.size.x = clip_bounds.size.x.ceil();
            clip_bounds.size.y = clip_bounds.size.y.ceil();

            let scissor_box = Some(ScissorBox {
                x: clip_bounds.position.x as i32,
                // Because OpenGL was designed for mathematicians, it has origin at lower left corner.
                y: viewport.size.y - (clip_bounds.position.y + clip_bounds.size.y) as i32,
                width: clip_bounds.size.x as i32,
                height: clip_bounds.size.y as i32,
            });

            let mut stencil_test = None;

            // Draw clipping geometry first if we have any. This is optional, because complex
            // clipping is very rare and in most cases scissor test will do the job.
            if let Some(clipping_geometry) = cmd.clipping_geometry.as_ref() {
                frame_buffer.clear(viewport, None, None, Some(0));

                self.clipping_geometry_buffer
                    .set_buffer_data_of_type(0, &clipping_geometry.vertex_buffer);
                self.clipping_geometry_buffer
                    .set_triangles(&clipping_geometry.triangle_buffer);

                // Draw
                let properties = PropertyGroup::from([property("worldViewProjection", &ortho)]);
                let material = RenderMaterial::from([binding("properties", &properties)]);
                statistics += renderer_resources.shaders.ui.run_pass(
                    1,
                    &ImmutableString::new("Clip"),
                    frame_buffer,
                    &self.geometry_buffer,
                    viewport,
                    &material,
                    uniform_buffer_cache,
                    Default::default(),
                    None,
                )?;

                // Make sure main geometry will be drawn only on marked pixels.
                stencil_test = Some(StencilFunc {
                    func: CompareFunc::Equal,
                    ref_value: 1,
                    ..Default::default()
                });
            }

            let params = DrawParameters {
                cull_face: None,
                color_write: ColorMask::all(true),
                depth_write: false,
                stencil_test,
                depth_test: None,
                blend: Some(BlendParameters {
                    func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                    ..Default::default()
                }),
                stencil_op: Default::default(),
                scissor_box,
            };

            let element_range = ElementRange::Specific {
                offset: cmd.triangles.start,
                count: cmd.triangles.end - cmd.triangles.start,
            };

            let material_data_guard = cmd.material.data_ref();
            let material = some_or_continue!(material_data_guard.as_loaded_ref());

            if let Some(render_pass_container) = render_pass_cache.get(server, material.shader()) {
                let shader_data_guard = material.shader().data_ref();
                let shader = some_or_continue!(shader_data_guard.as_loaded_ref());

                let render_pass = render_pass_container.get(&ImmutableString::new("Forward"))?;

                let mut resource_bindings = ArrayVec::<ResourceBinding, 32>::new();

                for resource in shader.definition.resources.iter() {
                    match resource.kind {
                        ShaderResourceKind::Texture { fallback, .. } => {
                            if resource.name.as_str() == "fyrox_widgetTexture" {
                                let mut diffuse_texture = (
                                    &renderer_resources.white_dummy,
                                    &renderer_resources.linear_wrap_sampler,
                                );

                                match &cmd.texture {
                                    CommandTexture::Font {
                                        font,
                                        page_index,
                                        height,
                                    } => {
                                        if let Some(font) = font.state().data() {
                                            let page_size = font.page_size() as u32;
                                            if let Some(page) = font
                                                .atlases
                                                .get_mut(height)
                                                .and_then(|atlas| atlas.pages.get_mut(*page_index))
                                            {
                                                if page.texture.is_none() || page.modified {
                                                    if let Some(details) = Texture::from_bytes(
                                                        TextureKind::Rectangle {
                                                            width: page_size,
                                                            height: page_size,
                                                        },
                                                        TexturePixelKind::R8,
                                                        page.pixels.clone(),
                                                    ) {
                                                        page.texture = Some(
                                                            TextureResource::new_ok(
                                                                Uuid::new_v4(),
                                                                ResourceKind::Embedded,
                                                                details,
                                                            )
                                                            .into(),
                                                        );
                                                        page.modified = false;
                                                    }
                                                }
                                                if let Some(texture) = texture_cache.get(
                                                    server,
                                                    resource_manager,
                                                    &page
                                                        .texture
                                                        .as_ref()
                                                        .unwrap()
                                                        .try_cast::<Texture>()
                                                        .unwrap(),
                                                ) {
                                                    diffuse_texture = (
                                                        &texture.gpu_texture,
                                                        &texture.gpu_sampler,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    CommandTexture::Texture(texture) => {
                                        if let Some(texture) =
                                            texture_cache.get(server, resource_manager, texture)
                                        {
                                            diffuse_texture =
                                                (&texture.gpu_texture, &texture.gpu_sampler);
                                        }
                                    }
                                    _ => (),
                                }

                                resource_bindings.push(ResourceBinding::texture(
                                    diffuse_texture.0,
                                    diffuse_texture.1,
                                    resource.binding,
                                ))
                            } else {
                                resource_bindings.push(make_texture_binding(
                                    server,
                                    material,
                                    resource,
                                    renderer_resources,
                                    fallback,
                                    resource_manager,
                                    texture_cache,
                                ))
                            }
                        }
                        ShaderResourceKind::PropertyGroup(_) => {
                            if let Some((_, block_location)) = command_uniform_blocks
                                .iter()
                                .find(|(binding, _)| *binding == resource.binding)
                            {
                                resource_bindings.push(
                                    uniform_memory_allocator
                                        .block_to_binding(*block_location, resource.binding),
                                );
                            }
                        }
                    }
                }

                statistics += frame_buffer.draw(
                    &self.geometry_buffer,
                    viewport,
                    &render_pass.program,
                    &params,
                    &[ResourceBindGroup {
                        bindings: &resource_bindings,
                    }],
                    element_range,
                )?;
            }
        }

        Ok(statistics)
    }
}
