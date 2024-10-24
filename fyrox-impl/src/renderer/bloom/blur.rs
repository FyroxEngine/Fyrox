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

use crate::{
    core::{algebra::Vector2, math::Rect, sstorage::ImmutableString},
    renderer::{
        cache::uniform::UniformBufferCache,
        framework::{
            error::FrameworkError,
            framebuffer::{
                Attachment, AttachmentKind, BufferLocation, FrameBuffer, ResourceBindGroup,
                ResourceBinding,
            },
            geometry_buffer::GeometryBuffer,
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{GpuTexture, PixelKind},
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            DrawParameters, ElementRange,
        },
        make_viewport_matrix, RenderPassStatistics,
    },
};
use std::{cell::RefCell, rc::Rc};

struct Shader {
    program: Box<dyn GpuProgram>,
    image: UniformLocation,
    uniform_block_binding: usize,
}

impl Shader {
    fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/gaussian_blur_fs.glsl");
        let vertex_source = include_str!("../shaders/gaussian_blur_vs.glsl");

        let program =
            server.create_program("GaussianBlurShader", vertex_source, fragment_source)?;
        Ok(Self {
            image: program.uniform_location(&ImmutableString::new("image"))?,
            uniform_block_binding: program
                .uniform_block_index(&ImmutableString::new("Uniforms"))?,
            program,
        })
    }
}

pub struct GaussianBlur {
    shader: Shader,
    h_framebuffer: Box<dyn FrameBuffer>,
    v_framebuffer: Box<dyn FrameBuffer>,
    width: usize,
    height: usize,
}

fn create_framebuffer(
    server: &dyn GraphicsServer,
    width: usize,
    height: usize,
    pixel_kind: PixelKind,
) -> Result<Box<dyn FrameBuffer>, FrameworkError> {
    let frame = server.create_2d_render_target(pixel_kind, width, height)?;

    server.create_frame_buffer(
        None,
        vec![Attachment {
            kind: AttachmentKind::Color,
            texture: frame,
        }],
    )
}

impl GaussianBlur {
    pub fn new(
        server: &dyn GraphicsServer,
        width: usize,
        height: usize,
        pixel_kind: PixelKind,
    ) -> Result<Self, FrameworkError> {
        Ok(Self {
            shader: Shader::new(server)?,
            h_framebuffer: create_framebuffer(server, width, height, pixel_kind)?,
            v_framebuffer: create_framebuffer(server, width, height, pixel_kind)?,
            width,
            height,
        })
    }

    fn h_blurred(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.h_framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn result(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.v_framebuffer.color_attachments()[0].texture.clone()
    }

    pub(crate) fn render(
        &mut self,
        quad: &dyn GeometryBuffer,
        input: Rc<RefCell<dyn GpuTexture>>,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.width as i32, self.height as i32);

        let inv_size = Vector2::new(1.0 / self.width as f32, 1.0 / self.height as f32);
        let shader = &self.shader;

        // Blur horizontally first.
        stats += self.h_framebuffer.draw(
            quad,
            viewport,
            &*shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: None,
                blend: None,
                stencil_op: Default::default(),
                scissor_box: None,
            },
            &[ResourceBindGroup {
                bindings: &[
                    ResourceBinding::texture(&input, &shader.image),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer_cache.write(
                            StaticUniformBuffer::<256>::new()
                                .with(&make_viewport_matrix(viewport))
                                .with(&inv_size)
                                .with(&true),
                        )?,
                        binding: BufferLocation::Auto {
                            shader_location: shader.uniform_block_binding,
                        },
                        data_usage: Default::default(),
                    },
                ],
            }],
            ElementRange::Full,
        )?;

        // Then blur vertically.
        let h_blurred_texture = self.h_blurred();
        stats += self.v_framebuffer.draw(
            quad,
            viewport,
            &*shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: None,
                blend: None,
                stencil_op: Default::default(),
                scissor_box: None,
            },
            &[ResourceBindGroup {
                bindings: &[
                    ResourceBinding::texture(&h_blurred_texture, &shader.image),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer_cache.write(
                            StaticUniformBuffer::<256>::new()
                                .with(&make_viewport_matrix(viewport))
                                .with(&inv_size)
                                .with(&false),
                        )?,
                        binding: BufferLocation::Auto {
                            shader_location: shader.uniform_block_binding,
                        },
                        data_usage: Default::default(),
                    },
                ],
            }],
            ElementRange::Full,
        )?;

        Ok(stats)
    }
}
