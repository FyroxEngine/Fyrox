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
    core::{
        algebra::{Matrix3, Matrix4, Vector2, Vector3},
        color::Color,
        math::{lerpf, Rect},
        sstorage::ImmutableString,
    },
    rand::Rng,
    renderer::{
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial, RenderPassContainer},
            uniform::UniformBufferCache,
        },
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, GpuFrameBuffer},
            geometry_buffer::GpuGeometryBuffer,
            gpu_texture::{GpuTexture, GpuTextureDescriptor, GpuTextureKind, PixelKind},
            server::GraphicsServer,
            GeometryBufferExt,
        },
        gbuffer::GBuffer,
        make_viewport_matrix,
        ssao::blur::Blur,
        RenderPassStatistics,
    },
    scene::mesh::surface::SurfaceData,
};

mod blur;

// Keep in sync with shader define.
const KERNEL_SIZE: usize = 32;

// Size of noise texture.
const NOISE_SIZE: usize = 4;

pub struct ScreenSpaceAmbientOcclusionRenderer {
    blur: Blur,
    program: RenderPassContainer,
    framebuffer: GpuFrameBuffer,
    quad: GpuGeometryBuffer,
    width: i32,
    height: i32,
    noise: GpuTexture,
    kernel: [Vector3<f32>; KERNEL_SIZE],
    radius: f32,
}

impl ScreenSpaceAmbientOcclusionRenderer {
    pub fn new(
        server: &dyn GraphicsServer,
        frame_width: usize,
        frame_height: usize,
    ) -> Result<Self, FrameworkError> {
        // It is good balance between quality and performance, no need to do SSAO in full resolution.
        // This SSAO map size reduction was taken from DOOM (2016).
        let width = (frame_width / 2).max(1);
        let height = (frame_height / 2).max(1);

        let occlusion = server.create_2d_render_target(PixelKind::R32F, width, height)?;

        let mut rng = crate::rand::thread_rng();

        Ok(Self {
            blur: Blur::new(server, width, height)?,
            program: RenderPassContainer::from_str(server, include_str!("../shaders/ssao.shader"))?,
            framebuffer: server.create_frame_buffer(
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: occlusion,
                }],
            )?,
            quad: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
            width: width as i32,
            height: height as i32,
            kernel: {
                let mut kernel = [Default::default(); KERNEL_SIZE];
                for (i, v) in kernel.iter_mut().enumerate() {
                    let k = i as f32 / KERNEL_SIZE as f32;
                    let scale = lerpf(0.1, 1.0, k * k);
                    *v = Vector3::new(
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(0.0..1.0),
                    )
                    // Make sphere
                    .try_normalize(f32::EPSILON)
                    .unwrap()
                    // Use non-uniform distribution to shuffle points inside hemisphere.
                    .scale(scale);
                }
                kernel
            },
            noise: {
                const RGB_PIXEL_SIZE: usize = 3;
                let mut pixels = [0u8; RGB_PIXEL_SIZE * NOISE_SIZE * NOISE_SIZE];
                for pixel in pixels.chunks_exact_mut(RGB_PIXEL_SIZE) {
                    pixel[0] = rng.gen_range(0u8..255u8); // R
                    pixel[1] = rng.gen_range(0u8..255u8); // G
                    pixel[2] = 0u8; // B
                }
                server.create_texture(GpuTextureDescriptor {
                    kind: GpuTextureKind::Rectangle {
                        width: NOISE_SIZE,
                        height: NOISE_SIZE,
                    },
                    pixel_kind: PixelKind::RGB8,
                    data: Some(&pixels),
                    ..Default::default()
                })?
            },
            radius: 0.5,
        })
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.abs();
    }

    fn raw_ao_map(&self) -> GpuTexture {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn ao_map(&self) -> GpuTexture {
        self.blur.result()
    }

    pub(crate) fn render(
        &mut self,
        gbuffer: &GBuffer,
        projection_matrix: Matrix4<f32>,
        view_matrix: Matrix3<f32>,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.width, self.height);

        let frame_matrix = make_viewport_matrix(viewport);

        self.framebuffer.clear(
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            None,
        );

        let noise_scale = Vector2::new(
            self.width as f32 / NOISE_SIZE as f32,
            self.height as f32 / NOISE_SIZE as f32,
        );

        let inv_projection = projection_matrix.try_inverse().unwrap_or_default();

        let properties = PropertyGroup::from([
            property("worldViewProjection", &frame_matrix),
            property("inverseProjectionMatrix", &inv_projection),
            property("projectionMatrix", &projection_matrix),
            property("kernel", self.kernel.as_slice()),
            property("noiseScale", &noise_scale),
            property("viewMatrix", &view_matrix),
            property("radius", &self.radius),
        ]);

        let material = RenderMaterial::from([
            binding(
                "depthSampler",
                (gbuffer.depth(), &fallback_resources.nearest_clamp_sampler),
            ),
            binding(
                "normalSampler",
                (
                    gbuffer.normal_texture(),
                    &fallback_resources.nearest_clamp_sampler,
                ),
            ),
            binding(
                "noiseSampler",
                (&self.noise, &fallback_resources.nearest_wrap_sampler),
            ),
            binding("properties", &properties),
        ]);

        stats += self.program.run_pass(
            1,
            &ImmutableString::new("Primary"),
            &self.framebuffer,
            &self.quad,
            viewport,
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )?;

        stats += self
            .blur
            .render(self.raw_ao_map(), uniform_buffer_cache, fallback_resources)?;

        Ok(stats)
    }
}
