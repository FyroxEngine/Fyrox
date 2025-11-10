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

//! Contains various algorithms for image convolution, primarily for lighting.

use crate::{
    core::{
        algebra::{Matrix4, Point3},
        color::Color,
        math::Rect,
        some_or_break, ImmutableString,
    },
    graphics::{
        error::FrameworkError,
        framebuffer::{Attachment, GpuFrameBuffer},
        gpu_texture::{GpuTexture, GpuTextureDescriptor, GpuTextureKind, PixelKind},
        server::GraphicsServer,
        stats::RenderPassStatistics,
        ElementRange,
    },
    renderer::{
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial},
            uniform::UniformBufferCache,
        },
        resources::RendererResources,
        utils::CubeMapFaceDescriptor,
    },
};

pub struct EnvironmentMapSpecularConvolution {
    framebuffer: GpuFrameBuffer,
    mip_count: usize,
    pub(crate) size: usize,
}

impl EnvironmentMapSpecularConvolution {
    pub fn new(server: &dyn GraphicsServer, size: usize) -> Result<Self, FrameworkError> {
        let mip_count = ((size as f32).log2().floor() + 1.0) as usize;
        let cube_map = server.create_texture(GpuTextureDescriptor {
            name: "EnvironmentMapSpecularConvolution",
            kind: GpuTextureKind::Cube { size },
            pixel_kind: PixelKind::RGB8,
            mip_count,
            data: None,
            base_level: 0,
            max_level: mip_count,
        })?;
        Ok(Self {
            framebuffer: server.create_frame_buffer(None, vec![Attachment::color(cube_map)])?,
            mip_count,
            size,
        })
    }

    pub fn cube_map(&self) -> &GpuTexture {
        &self.framebuffer.color_attachments()[0].texture
    }

    pub fn render(
        &self,
        server: &dyn GraphicsServer,
        environment_map: &GpuTexture,
        uniform_buffer_cache: &mut UniformBufferCache,
        renderer_resources: &RendererResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let _debug_scope = server.begin_scope("EnvironmentMapSpecularConvolution");

        let mut stats = RenderPassStatistics::default();

        let projection_matrix =
            Matrix4::new_perspective(1.0, std::f32::consts::FRAC_PI_2, 0.0125, 32.0);

        for mip in 0..self.mip_count {
            let roughness = (mip as f32) / (self.mip_count - 1) as f32;
            let size = some_or_break!(self.size.checked_shr(mip as u32));
            let viewport = Rect::new(0, 0, size as i32, size as i32);
            for face in CubeMapFaceDescriptor::cube_faces() {
                self.framebuffer.set_cubemap_face(0, face.face, mip);
                self.framebuffer
                    .clear(viewport, Some(Color::WHITE), None, None);

                let view_matrix =
                    Matrix4::look_at_rh(&Default::default(), &Point3::from(face.look), &face.up);

                let wvp = projection_matrix * view_matrix;
                let properties = PropertyGroup::from([
                    property("worldViewProjection", &wvp),
                    property("roughness", &roughness),
                ]);
                let material = RenderMaterial::from([
                    binding(
                        "environmentMap",
                        (environment_map, &renderer_resources.linear_clamp_sampler),
                    ),
                    binding("properties", &properties),
                ]);

                stats += renderer_resources
                    .shaders
                    .environment_map_specular_convolution
                    .run_pass(
                        1,
                        &ImmutableString::new("Primary"),
                        &self.framebuffer,
                        &renderer_resources.cube,
                        viewport,
                        &material,
                        uniform_buffer_cache,
                        ElementRange::Full,
                        None,
                    )?;
            }
        }

        Ok(stats)
    }
}

pub struct EnvironmentMapIrradianceConvolution {
    framebuffer: GpuFrameBuffer,
    size: usize,
}

impl EnvironmentMapIrradianceConvolution {
    pub fn new(server: &dyn GraphicsServer, size: usize) -> Result<Self, FrameworkError> {
        let cube_map = server.create_texture(GpuTextureDescriptor {
            name: "EnvironmentMapIrradianceConvolution",
            kind: GpuTextureKind::Cube { size },
            pixel_kind: PixelKind::RGB8,
            mip_count: 1,
            data: None,
            base_level: 0,
            max_level: 1,
        })?;
        Ok(Self {
            framebuffer: server.create_frame_buffer(None, vec![Attachment::color(cube_map)])?,
            size,
        })
    }

    pub fn cube_map(&self) -> &GpuTexture {
        &self.framebuffer.color_attachments()[0].texture
    }

    pub fn render(
        &self,
        server: &dyn GraphicsServer,
        environment_map: &GpuTexture,
        uniform_buffer_cache: &mut UniformBufferCache,
        renderer_resources: &RendererResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let _debug_scope = server.begin_scope("EnvironmentMapIrradianceConvolution");

        let mut stats = RenderPassStatistics::default();

        let projection_matrix =
            Matrix4::new_perspective(1.0, std::f32::consts::FRAC_PI_2, 0.0125, 32.0);

        let viewport = Rect::new(0, 0, self.size as i32, self.size as i32);
        for face in CubeMapFaceDescriptor::cube_faces() {
            self.framebuffer.set_cubemap_face(0, face.face, 0);
            self.framebuffer
                .clear(viewport, Some(Color::WHITE), None, None);

            let view_matrix =
                Matrix4::look_at_rh(&Default::default(), &Point3::from(face.look), &face.up);

            let wvp = projection_matrix * view_matrix;
            let properties = PropertyGroup::from([property("worldViewProjection", &wvp)]);
            let material = RenderMaterial::from([
                binding(
                    "environmentMap",
                    (environment_map, &renderer_resources.linear_clamp_sampler),
                ),
                binding("properties", &properties),
            ]);

            stats += renderer_resources
                .shaders
                .environment_map_irradiance_convolution
                .run_pass(
                    1,
                    &ImmutableString::new("Primary"),
                    &self.framebuffer,
                    &renderer_resources.cube,
                    viewport,
                    &material,
                    uniform_buffer_cache,
                    ElementRange::Full,
                    None,
                )?;
        }

        Ok(stats)
    }
}
