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

use crate::renderer::flat_shader::FlatShader;
use crate::{
    asset::untyped::ResourceKind,
    core::{
        algebra::{Matrix4, Vector2, Vector4},
        color::Color,
        math::Rect,
        sstorage::ImmutableString,
    },
    gui::{
        brush::Brush,
        draw::{CommandTexture, DrawingContext},
    },
    renderer::{
        cache::uniform::UniformBufferCache,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::FrameBuffer,
            geometry_buffer::{
                AttributeDefinition, AttributeKind, BufferBuilder, GeometryBuffer,
                GeometryBufferBuilder,
            },
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::GpuTexture,
            state::GlGraphicsServer,
            BlendFactor, BlendFunc, BlendParameters, ColorMask, CompareFunc, DrawParameters,
            ElementKind, ElementRange, ScissorBox, StencilAction, StencilFunc, StencilOp,
        },
        RenderPassStatistics, TextureCache,
    },
    resource::texture::{Texture, TextureKind, TexturePixelKind, TextureResource},
};
use fyrox_graphics::uniform::StaticUniformBuffer;
use std::{cell::RefCell, rc::Rc};

struct UiShader {
    program: GpuProgram,
    diffuse_texture: UniformLocation,
    uniform_block_index: usize,
}

impl UiShader {
    pub fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/ui_fs.glsl");
        let vertex_source = include_str!("shaders/ui_vs.glsl");
        let program = GpuProgram::from_source(server, "UIShader", vertex_source, fragment_source)?;
        Ok(Self {
            diffuse_texture: program
                .uniform_location(server, &ImmutableString::new("diffuseTexture"))?,
            uniform_block_index: program
                .uniform_block_index(server, &ImmutableString::new("Uniforms"))?,
            program,
        })
    }
}

/// User interface renderer allows you to render drawing context in specified render target.
pub struct UiRenderer {
    shader: UiShader,
    geometry_buffer: GeometryBuffer,
    clipping_geometry_buffer: GeometryBuffer,
}

/// A set of parameters to render a specified user interface drawing context.
pub struct UiRenderContext<'a, 'b, 'c> {
    /// Render pipeline state.
    pub state: &'a GlGraphicsServer,
    /// Viewport to where render the user interface.
    pub viewport: Rect<i32>,
    /// Frame buffer to where render the user interface.
    pub frame_buffer: &'b mut FrameBuffer,
    /// Width of the frame buffer to where render the user interface.
    pub frame_width: f32,
    /// Height of the frame buffer to where render the user interface.
    pub frame_height: f32,
    /// Drawing context of a user interface.
    pub drawing_context: &'c DrawingContext,
    /// A reference of white-pixel texture.
    pub white_dummy: Rc<RefCell<dyn GpuTexture>>,
    /// GPU texture cache.
    pub texture_cache: &'a mut TextureCache,
    /// A reference to the cache of uniform buffers.
    pub uniform_buffer_cache: &'a mut UniformBufferCache,
    /// A reference to the shader that will be used to draw clipping geometry.
    pub flat_shader: &'a FlatShader,
}

impl UiRenderer {
    pub(in crate::renderer) fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let geometry_buffer = GeometryBufferBuilder::new(ElementKind::Triangle)
            .with_buffer_builder(
                BufferBuilder::new::<crate::gui::draw::Vertex>(BufferUsage::DynamicDraw, None)
                    .with_attribute(AttributeDefinition {
                        location: 0,
                        kind: AttributeKind::Float2,
                        normalized: false,
                        divisor: 0,
                    })
                    .with_attribute(AttributeDefinition {
                        location: 1,
                        kind: AttributeKind::Float2,
                        normalized: false,
                        divisor: 0,
                    })
                    .with_attribute(AttributeDefinition {
                        location: 2,
                        kind: AttributeKind::UnsignedByte4,
                        normalized: true, // Make sure [0; 255] -> [0; 1]
                        divisor: 0,
                    }),
            )
            .build(server)?;

        let clipping_geometry_buffer = GeometryBufferBuilder::new(ElementKind::Triangle)
            .with_buffer_builder(
                BufferBuilder::new::<crate::gui::draw::Vertex>(BufferUsage::DynamicDraw, None)
                    // We're interested only in position. Fragment shader won't run for clipping geometry anyway.
                    .with_attribute(AttributeDefinition {
                        location: 0,
                        kind: AttributeKind::Float2,
                        normalized: false,
                        divisor: 0,
                    }),
            )
            .build(server)?;

        Ok(Self {
            geometry_buffer,
            clipping_geometry_buffer,
            shader: UiShader::new(server)?,
        })
    }

    /// Renders given UI's drawing context to specified frame buffer.
    pub fn render(
        &mut self,
        args: UiRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let UiRenderContext {
            state: server,
            viewport,
            frame_buffer,
            frame_width,
            frame_height,
            drawing_context,
            white_dummy,
            texture_cache,
            uniform_buffer_cache,
            flat_shader,
        } = args;

        let mut statistics = RenderPassStatistics::default();

        self.geometry_buffer
            .set_buffer_data(0, drawing_context.get_vertices());

        let geometry_buffer = self.geometry_buffer.bind(server);
        geometry_buffer.set_triangles(drawing_context.get_triangles());

        let ortho = Matrix4::new_orthographic(0.0, frame_width, frame_height, 0.0, -1.0, 1.0);
        let resolution = Vector2::new(frame_width, frame_height);

        for cmd in drawing_context.get_commands() {
            let mut diffuse_texture = &white_dummy;
            let mut is_font_texture = false;

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
                    .set_buffer_data(0, &clipping_geometry.vertex_buffer);
                self.clipping_geometry_buffer
                    .bind(server)
                    .set_triangles(&clipping_geometry.triangle_buffer);

                // Draw
                statistics += frame_buffer.draw(
                    &self.clipping_geometry_buffer,
                    viewport,
                    &flat_shader.program,
                    &DrawParameters {
                        cull_face: None,
                        color_write: ColorMask::all(false),
                        depth_write: false,
                        stencil_test: None,
                        depth_test: None,
                        blend: None,
                        stencil_op: StencilOp {
                            zpass: StencilAction::Incr,
                            ..Default::default()
                        },
                        scissor_box,
                    },
                    ElementRange::Full,
                    &mut |mut program_binding| {
                        program_binding.set_matrix4(&flat_shader.wvp_matrix, &ortho);
                    },
                )?;

                // Make sure main geometry will be drawn only on marked pixels.
                stencil_test = Some(StencilFunc {
                    func: CompareFunc::Equal,
                    ref_value: 1,
                    ..Default::default()
                });
            }

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
                                        TextureResource::new_ok(ResourceKind::Embedded, details)
                                            .into(),
                                    );
                                    page.modified = false;
                                }
                            }
                            if let Some(texture) = texture_cache.get(
                                server,
                                &page
                                    .texture
                                    .as_ref()
                                    .unwrap()
                                    .try_cast::<Texture>()
                                    .unwrap(),
                            ) {
                                diffuse_texture = texture;
                            }
                            is_font_texture = true;
                        }
                    }
                }
                CommandTexture::Texture(texture) => {
                    if let Some(resource) = texture.try_cast::<Texture>() {
                        if let Some(texture) = texture_cache.get(server, &resource) {
                            diffuse_texture = texture;
                        }
                    }
                }
                _ => (),
            }

            let mut raw_stops = [0.0; 16];
            let mut raw_colors = [Vector4::default(); 16];
            let bounds_max = cmd.bounds.right_bottom_corner();

            let (gradient_origin, gradient_end) = match cmd.brush {
                Brush::Solid(_) => (Vector2::default(), Vector2::default()),
                Brush::LinearGradient { from, to, .. } => (from, to),
                Brush::RadialGradient { center, .. } => (center, Vector2::default()),
            };

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

            let mut uniforms = StaticUniformBuffer::<1024>::new();
            uniforms
                .push(&ortho)
                .push(&solid_color)
                .push_slice(gradient_colors)
                .push_slice(gradient_stops)
                .push(&gradient_origin)
                .push(&gradient_end)
                .push(&resolution)
                .push(&cmd.bounds.position)
                .push(&bounds_max)
                .push(&is_font_texture)
                .push(&cmd.opacity)
                .push(&brush_type)
                .push(&gradient_point_count);
            let uniforms_bytes = uniforms.finish();
            let uniform_buffer =
                uniform_buffer_cache.get_or_create(server, uniforms_bytes.len())?;
            uniform_buffer.write_data(&uniforms_bytes)?;

            let shader = &self.shader;
            statistics += frame_buffer.draw(
                &self.geometry_buffer,
                viewport,
                &self.shader.program,
                &params,
                ElementRange::Specific {
                    offset: cmd.triangles.start,
                    count: cmd.triangles.end - cmd.triangles.start,
                },
                &mut |mut program_binding| {
                    program_binding
                        .set_texture(&shader.diffuse_texture, diffuse_texture)
                        .bind_uniform_buffer(
                            uniform_buffer,
                            self.shader.uniform_block_index as u32,
                            0,
                        );
                },
            )?;
        }

        Ok(statistics)
    }
}
