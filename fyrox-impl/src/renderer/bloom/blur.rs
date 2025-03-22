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
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, GpuFrameBuffer},
            geometry_buffer::GpuGeometryBuffer,
            gpu_texture::{GpuTexture, PixelKind},
            server::GraphicsServer,
        },
        make_viewport_matrix, RenderPassStatistics,
    },
};

pub struct GaussianBlur {
    shader: RenderPassContainer,
    h_framebuffer: GpuFrameBuffer,
    v_framebuffer: GpuFrameBuffer,
    width: usize,
    height: usize,
}

fn create_framebuffer(
    server: &dyn GraphicsServer,
    width: usize,
    height: usize,
    pixel_kind: PixelKind,
) -> Result<GpuFrameBuffer, FrameworkError> {
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
            shader: RenderPassContainer::from_str(
                server,
                include_str!("../shaders/gaussian_blur.shader"),
            )?,
            h_framebuffer: create_framebuffer(server, width, height, pixel_kind)?,
            v_framebuffer: create_framebuffer(server, width, height, pixel_kind)?,
            width,
            height,
        })
    }

    fn h_blurred(&self) -> &GpuTexture {
        &self.h_framebuffer.color_attachments()[0].texture
    }

    pub fn result(&self) -> &GpuTexture {
        &self.v_framebuffer.color_attachments()[0].texture
    }

    pub(crate) fn render(
        &self,
        quad: &GpuGeometryBuffer,
        input: &GpuTexture,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.width as i32, self.height as i32);
        let inv_size = Vector2::new(1.0 / self.width as f32, 1.0 / self.height as f32);
        let wvp = make_viewport_matrix(viewport);

        for (image, framebuffer, horizontal) in [
            (input, &self.h_framebuffer, true),
            (self.h_blurred(), &self.v_framebuffer, false),
        ] {
            let properties = PropertyGroup::from([
                property("worldViewProjection", &wvp),
                property("pixelSize", &inv_size),
                property("horizontal", &horizontal),
            ]);
            let material = RenderMaterial::from([
                binding("image", (image, &fallback_resources.nearest_clamp_sampler)),
                binding("properties", &properties),
            ]);

            stats += self.shader.run_pass(
                1,
                &ImmutableString::new("Primary"),
                framebuffer,
                quad,
                viewport,
                &material,
                uniform_buffer_cache,
                Default::default(),
                None,
            )?;
        }

        Ok(stats)
    }
}
