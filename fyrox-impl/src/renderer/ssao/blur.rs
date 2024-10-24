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
        cache::uniform::UniformBufferCache,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{
                Attachment, AttachmentKind, BufferLocation, FrameBuffer, ResourceBindGroup,
                ResourceBinding,
            },
            geometry_buffer::{DrawCallStatistics, GeometryBuffer},
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{GpuTexture, PixelKind},
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            DrawParameters, ElementRange, GeometryBufferExt,
        },
        make_viewport_matrix,
    },
    scene::mesh::surface::SurfaceData,
};
use std::{cell::RefCell, rc::Rc};

struct Shader {
    program: Box<dyn GpuProgram>,
    input_texture: UniformLocation,
    uniform_buffer_binding: usize,
}

impl Shader {
    fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/blur_fs.glsl");
        let vertex_source = include_str!("../shaders/blur_vs.glsl");

        let program = server.create_program("BlurShader", vertex_source, fragment_source)?;
        Ok(Self {
            uniform_buffer_binding: program
                .uniform_block_index(&ImmutableString::new("Uniforms"))?,
            input_texture: program.uniform_location(&ImmutableString::new("inputTexture"))?,
            program,
        })
    }
}

pub struct Blur {
    shader: Shader,
    framebuffer: Box<dyn FrameBuffer>,
    quad: Box<dyn GeometryBuffer>,
    width: usize,
    height: usize,
}

impl Blur {
    pub fn new(
        server: &dyn GraphicsServer,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        let frame = server.create_2d_render_target(PixelKind::R32F, width, height)?;

        Ok(Self {
            shader: Shader::new(server)?,
            framebuffer: server.create_frame_buffer(
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: frame,
                }],
            )?,
            quad: <dyn GeometryBuffer>::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
            width,
            height,
        })
    }

    pub fn result(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub(crate) fn render(
        &mut self,
        input: Rc<RefCell<dyn GpuTexture>>,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let viewport = Rect::new(0, 0, self.width as i32, self.height as i32);

        let shader = &self.shader;
        self.framebuffer.draw(
            &*self.quad,
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
                    ResourceBinding::texture(&input, &shader.input_texture),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer_cache.write(
                            StaticUniformBuffer::<256>::new().with(&make_viewport_matrix(viewport)),
                        )?,
                        binding: BufferLocation::Auto {
                            shader_location: shader.uniform_buffer_binding,
                        },
                        data_usage: Default::default(),
                    },
                ],
            }],
            ElementRange::Full,
        )
    }
}
