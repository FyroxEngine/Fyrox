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

use crate::renderer::FallbackResources;
use crate::{
    core::{math::Rect, ImmutableString},
    renderer::{
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial, RenderPassContainer},
            uniform::UniformBufferCache,
        },
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{Attachment, DrawCallStatistics, GpuFrameBuffer},
            geometry_buffer::GpuGeometryBuffer,
            gpu_texture::{GpuTexture, PixelKind},
            server::GraphicsServer,
            GeometryBufferExt,
        },
        make_viewport_matrix,
    },
    scene::mesh::surface::SurfaceData,
};

pub struct Blur {
    program: RenderPassContainer,
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
        let program =
            RenderPassContainer::from_str(server, include_str!("../shaders/blur.shader"))?;
        Ok(Self {
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
        fallback_resources: &FallbackResources,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let viewport = Rect::new(0, 0, self.width as i32, self.height as i32);

        let wvp = make_viewport_matrix(viewport);
        let properties = PropertyGroup::from([property("worldViewProjection", &wvp)]);
        let material = RenderMaterial::from([
            binding(
                "inputTexture",
                (&input, &fallback_resources.nearest_clamp_sampler),
            ),
            binding("properties", &properties),
        ]);

        self.program.run_pass(
            1,
            &ImmutableString::new("Primary"),
            &self.framebuffer,
            &self.quad,
            viewport,
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )
    }
}
