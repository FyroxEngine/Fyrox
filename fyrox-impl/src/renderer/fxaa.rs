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
    renderer::make_viewport_matrix,
    renderer::{
        cache::uniform::UniformBufferCache,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{BufferLocation, FrameBuffer, ResourceBindGroup, ResourceBinding},
            geometry_buffer::GeometryBuffer,
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::GpuTexture,
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            DrawParameters, ElementRange, GeometryBufferExt,
        },
        RenderPassStatistics,
    },
    scene::mesh::surface::SurfaceData,
};
use std::{cell::RefCell, rc::Rc};

struct FxaaShader {
    pub program: Box<dyn GpuProgram>,
    pub uniform_buffer_binding: usize,
    pub screen_texture: UniformLocation,
}

impl FxaaShader {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/fxaa_fs.glsl");
        let vertex_source = include_str!("shaders/fxaa_vs.glsl");

        let program = server.create_program("FXAAShader", vertex_source, fragment_source)?;
        Ok(Self {
            uniform_buffer_binding: program
                .uniform_block_index(&ImmutableString::new("Uniforms"))?,
            screen_texture: program.uniform_location(&ImmutableString::new("screenTexture"))?,
            program,
        })
    }
}

pub struct FxaaRenderer {
    shader: FxaaShader,
    quad: Box<dyn GeometryBuffer>,
}

impl FxaaRenderer {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            shader: FxaaShader::new(server)?,
            quad: <dyn GeometryBuffer>::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
        })
    }

    pub(crate) fn render(
        &self,
        viewport: Rect<i32>,
        frame_texture: Rc<RefCell<dyn GpuTexture>>,
        frame_buffer: &mut dyn FrameBuffer,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

        let frame_matrix = make_viewport_matrix(viewport);

        statistics += frame_buffer.draw(
            &*self.quad,
            viewport,
            &*self.shader.program,
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
                    ResourceBinding::texture(&frame_texture, &self.shader.screen_texture),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer_cache.write(
                            StaticUniformBuffer::<256>::new().with(&frame_matrix).with(
                                &Vector2::new(1.0 / viewport.w() as f32, 1.0 / viewport.h() as f32),
                            ),
                        )?,
                        binding: BufferLocation::Auto {
                            shader_location: self.shader.uniform_buffer_binding,
                        },
                        data_usage: Default::default(),
                    },
                ],
            }],
            ElementRange::Full,
        )?;

        Ok(statistics)
    }
}
