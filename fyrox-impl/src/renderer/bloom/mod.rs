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
        bloom::blur::GaussianBlur,
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial, RenderPassContainer},
            uniform::UniformBufferCache,
        },
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, GpuFrameBuffer},
            geometry_buffer::GpuGeometryBuffer,
            gpu_texture::{GpuTexture, PixelKind},
            server::GraphicsServer,
        },
        make_viewport_matrix, RenderPassStatistics,
    },
};

mod blur;

pub struct BloomRenderer {
    shader: RenderPassContainer,
    framebuffer: GpuFrameBuffer,
    blur: GaussianBlur,
    width: usize,
    height: usize,
}

impl BloomRenderer {
    pub fn new(
        server: &dyn GraphicsServer,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        Ok(Self {
            shader: RenderPassContainer::from_str(server, include_str!("../shaders/bloom.shader"))?,
            blur: GaussianBlur::new(server, width, height, PixelKind::RGBA16F)?,
            framebuffer: server.create_frame_buffer(
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: server.create_2d_render_target(PixelKind::RGBA16F, width, height)?,
                }],
            )?,
            width,
            height,
        })
    }

    fn glow_texture(&self) -> &GpuTexture {
        &self.framebuffer.color_attachments()[0].texture
    }

    pub fn result(&self) -> &GpuTexture {
        self.blur.result()
    }

    pub(crate) fn render(
        &self,
        quad: &GpuGeometryBuffer,
        hdr_scene_frame: &GpuTexture,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.width as i32, self.height as i32);

        let wvp = make_viewport_matrix(viewport);
        let properties = PropertyGroup::from([property("worldViewProjection", &wvp)]);
        let material = RenderMaterial::from([
            binding(
                "hdrSampler",
                (hdr_scene_frame, &fallback_resources.nearest_clamp_sampler),
            ),
            binding("properties", &properties),
        ]);

        stats += self.shader.run_pass(
            1,
            &ImmutableString::new("Primary"),
            &self.framebuffer,
            quad,
            viewport,
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )?;

        stats += self.blur.render(
            quad,
            self.glow_texture(),
            uniform_buffer_cache,
            fallback_resources,
        )?;

        Ok(stats)
    }
}
