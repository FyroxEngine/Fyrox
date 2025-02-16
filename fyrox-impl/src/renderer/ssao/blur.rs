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
    core::math::Rect,
    material::shader::Shader,
    renderer::{
        cache::uniform::UniformBufferCache,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{
                Attachment, DrawCallStatistics, GpuFrameBuffer, ResourceBindGroup, ResourceBinding,
            },
            geometry_buffer::GpuGeometryBuffer,
            gpu_program::GpuProgram,
            gpu_texture::{GpuTexture, PixelKind},
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            ElementRange, GeometryBufferExt,
        },
        make_viewport_matrix,
    },
    scene::mesh::surface::SurfaceData,
};

pub struct Blur {
    shader: Shader,
    program: GpuProgram,
    framebuffer: GpuFrameBuffer,
    quad: GpuGeometryBuffer,
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

        let shader = Shader::from_string(include_str!("../shaders/blur.shader"))
            .map_err(|e| FrameworkError::Custom(e.to_string()))?;
        let pass = &shader.definition.passes[0];

        let program = server.create_program_with_properties(
            "BlurShader",
            &pass.vertex_shader,
            &pass.fragment_shader,
            &shader.definition.resources,
        )?;

        Ok(Self {
            shader,
            program,
            framebuffer: server.create_frame_buffer(None, vec![Attachment::color(frame)])?,
            quad: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
            width,
            height,
        })
    }

    pub fn result(&self) -> GpuTexture {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub(crate) fn render(
        &mut self,
        input: GpuTexture,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let viewport = Rect::new(0, 0, self.width as i32, self.height as i32);

        let uniforms = uniform_buffer_cache
            .write(StaticUniformBuffer::<256>::new().with(&make_viewport_matrix(viewport)))?;

        self.framebuffer.draw(
            &*self.quad,
            viewport,
            &*self.program,
            &self.shader.definition.passes[0].draw_parameters,
            &[ResourceBindGroup {
                bindings: &[
                    ResourceBinding::texture_with_binding(&input, 0),
                    ResourceBinding::buffer_with_binding(&uniforms, 0, Default::default()),
                ],
            }],
            ElementRange::Full,
        )
    }
}
