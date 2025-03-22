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
    core::{algebra::Vector2, math::Rect, sstorage::ImmutableString},
    renderer::{
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial, RenderPassContainer},
            uniform::UniformBufferCache,
        },
        framework::{
            buffer::BufferUsage, error::FrameworkError, framebuffer::GpuFrameBuffer,
            geometry_buffer::GpuGeometryBuffer, gpu_texture::GpuTexture, server::GraphicsServer,
            GeometryBufferExt,
        },
        make_viewport_matrix, RenderPassStatistics,
    },
    scene::mesh::surface::SurfaceData,
};

pub struct FxaaRenderer {
    shader: RenderPassContainer,
    quad: GpuGeometryBuffer,
}

impl FxaaRenderer {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            shader: RenderPassContainer::from_str(server, include_str!("shaders/fxaa.shader"))?,
            quad: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
        })
    }

    pub(crate) fn render(
        &self,
        viewport: Rect<i32>,
        frame_texture: &GpuTexture,
        frame_buffer: &GpuFrameBuffer,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

        let frame_matrix = make_viewport_matrix(viewport);

        let inv_screen_size = Vector2::new(1.0 / viewport.w() as f32, 1.0 / viewport.h() as f32);
        let properties = PropertyGroup::from([
            property("worldViewProjection", &frame_matrix),
            property("inverseScreenSize", &inv_screen_size),
        ]);
        let material = RenderMaterial::from([
            binding(
                "screenTexture",
                (frame_texture, &fallback_resources.nearest_clamp_sampler),
            ),
            binding("properties", &properties),
        ]);

        statistics += self.shader.run_pass(
            1,
            &ImmutableString::new("Primary"),
            frame_buffer,
            &self.quad,
            viewport,
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )?;

        Ok(statistics)
    }
}
