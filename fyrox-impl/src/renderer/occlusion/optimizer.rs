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
    core::{color::Color, math::Rect, ImmutableString},
    graphics::{
        error::FrameworkError,
        framebuffer::{Attachment, GpuFrameBuffer},
        gpu_texture::{GpuTexture, PixelKind},
        read_buffer::GpuAsyncReadBuffer,
        server::GraphicsServer,
        stats::RenderPassStatistics,
    },
    renderer::resources::RendererResources,
    renderer::{
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial},
            uniform::UniformBufferCache,
        },
        make_viewport_matrix,
    },
};

pub struct VisibilityBufferOptimizer {
    framebuffer: GpuFrameBuffer,
    pixel_buffer: GpuAsyncReadBuffer,

    w_tiles: usize,
    h_tiles: usize,
}

impl VisibilityBufferOptimizer {
    pub fn new(
        server: &dyn GraphicsServer,
        w_tiles: usize,
        h_tiles: usize,
    ) -> Result<Self, FrameworkError> {
        let optimized_visibility_buffer = server.create_2d_render_target(
            "OptimizedVisibilityTexture",
            PixelKind::R32UI,
            w_tiles,
            h_tiles,
        )?;

        Ok(Self {
            framebuffer: server
                .create_frame_buffer(None, vec![Attachment::color(optimized_visibility_buffer)])?,
            pixel_buffer: server.create_async_read_buffer(
                "OcclusionReadBuffer",
                size_of::<u32>(),
                w_tiles * h_tiles,
            )?,
            w_tiles,
            h_tiles,
        })
    }

    pub fn is_reading_from_gpu(&self) -> bool {
        self.pixel_buffer.is_request_running()
    }

    pub fn read_visibility_mask(&mut self) -> Option<Vec<u32>> {
        self.pixel_buffer.try_read_of_type()
    }

    pub fn optimize(
        &mut self,
        visibility_buffer: &GpuTexture,
        tile_size: i32,
        uniform_buffer_cache: &mut UniformBufferCache,
        renderer_resources: &RendererResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.w_tiles as i32, self.h_tiles as i32);

        self.framebuffer
            .clear(viewport, Some(Color::TRANSPARENT), None, None);

        let matrix = make_viewport_matrix(viewport);
        let properties = PropertyGroup::from([
            property("viewProjection", &matrix),
            property("tileSize", &tile_size),
        ]);
        let material = RenderMaterial::from([
            binding(
                "visibilityBuffer",
                (visibility_buffer, &renderer_resources.nearest_clamp_sampler),
            ),
            binding("properties", &properties),
        ]);

        stats += renderer_resources.shaders.visibility_optimizer.run_pass(
            1,
            &ImmutableString::new("Primary"),
            &self.framebuffer,
            &renderer_resources.quad,
            viewport,
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )?;

        self.pixel_buffer
            .schedule_pixels_transfer(&*self.framebuffer, 0, None)?;

        Ok(stats)
    }
}
