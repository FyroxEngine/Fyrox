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
    core::{math::Rect, sstorage::ImmutableString},
    renderer::{
        bloom::blur::GaussianBlur,
        cache::uniform::UniformBufferCache,
        framework::{
            error::FrameworkError,
            framebuffer::{
                Attachment, AttachmentKind, FrameBuffer, ResourceBindGroup, ResourceBinding,
            },
            geometry_buffer::GeometryBuffer,
            gl::server::GlGraphicsServer,
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            DrawParameters, ElementRange,
        },
        make_viewport_matrix, RenderPassStatistics,
    },
};
use std::{cell::RefCell, rc::Rc};

mod blur;

struct Shader {
    program: GpuProgram,
    uniform_block_binding: usize,
    hdr_sampler: UniformLocation,
}

impl Shader {
    fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/bloom_fs.glsl");
        let vertex_source = include_str!("../shaders/bloom_vs.glsl");

        let program =
            GpuProgram::from_source(server, "BloomShader", vertex_source, fragment_source)?;
        Ok(Self {
            uniform_block_binding: program
                .uniform_block_index(server, &ImmutableString::new("Uniforms"))?,
            hdr_sampler: program.uniform_location(server, &ImmutableString::new("hdrSampler"))?,
            program,
        })
    }
}

pub struct BloomRenderer {
    shader: Shader,
    framebuffer: Box<dyn FrameBuffer>,
    blur: GaussianBlur,
    width: usize,
    height: usize,
}

impl BloomRenderer {
    pub fn new(
        server: &GlGraphicsServer,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        let frame = {
            let kind = GpuTextureKind::Rectangle { width, height };
            let texture = server.create_texture(
                kind,
                PixelKind::RGBA16F,
                MinificationFilter::Nearest,
                MagnificationFilter::Nearest,
                1,
                None,
            )?;
            texture
                .borrow_mut()
                .set_wrap(Coordinate::S, WrapMode::ClampToEdge);
            texture
                .borrow_mut()
                .set_wrap(Coordinate::T, WrapMode::ClampToEdge);
            texture
        };

        Ok(Self {
            shader: Shader::new(server)?,
            blur: GaussianBlur::new(server, width, height, PixelKind::RGBA16F)?,
            framebuffer: server.create_frame_buffer(
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: frame,
                }],
            )?,
            width,
            height,
        })
    }

    fn glow_texture(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn result(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.blur.result()
    }

    pub(crate) fn render(
        &mut self,
        server: &dyn GraphicsServer,
        quad: &GeometryBuffer,
        hdr_scene_frame: Rc<RefCell<dyn GpuTexture>>,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.width as i32, self.height as i32);

        let shader = &self.shader;
        stats += self.framebuffer.draw(
            quad,
            viewport,
            &shader.program,
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
                    ResourceBinding::texture(&hdr_scene_frame, &shader.hdr_sampler),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer_cache.write(
                            server,
                            StaticUniformBuffer::<256>::new().with(&make_viewport_matrix(viewport)),
                        )?,
                        shader_location: shader.uniform_block_binding,
                    },
                ],
            }],
            ElementRange::Full,
            &mut |_| {},
        )?;

        stats += self
            .blur
            .render(server, quad, self.glow_texture(), uniform_buffer_cache)?;

        Ok(stats)
    }
}
