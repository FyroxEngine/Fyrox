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
        algebra::{Matrix4, Vector2},
        color::Color,
        math::Rect,
        value_as_u8_slice, ImmutableString,
    },
    renderer::{
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial, RenderPassContainer},
            texture::TextureCache,
            uniform::UniformBufferCache,
        },
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, DrawCallStatistics, GpuFrameBuffer},
            geometry_buffer::GpuGeometryBuffer,
            gpu_texture::{GpuTexture, GpuTextureDescriptor, GpuTextureKind, PixelKind},
            server::GraphicsServer,
        },
        hdr::{adaptation::AdaptationChain, luminance::luminance_evaluator::LuminanceEvaluator},
        make_viewport_matrix, RenderPassStatistics,
    },
    scene::camera::{ColorGradingLut, Exposure},
};
use fyrox_graphics::framebuffer::ReadTarget;

mod adaptation;
mod luminance;

#[allow(dead_code)] // TODO
pub enum LuminanceCalculationMethod {
    Histogram,
    DownSampling,
}

pub struct LumBuffer {
    framebuffer: GpuFrameBuffer,
    size: usize,
}

impl LumBuffer {
    fn new(server: &dyn GraphicsServer, size: usize) -> Result<Self, FrameworkError> {
        let texture = server.create_2d_render_target(PixelKind::R32F, size, size)?;
        Ok(Self {
            framebuffer: server.create_frame_buffer(
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture,
                }],
            )?,
            size,
        })
    }

    fn clear(&self) {
        self.framebuffer.clear(
            Rect::new(0, 0, self.size as i32, self.size as i32),
            Some(Color::BLACK),
            None,
            None,
        );
    }

    fn matrix(&self) -> Matrix4<f32> {
        make_viewport_matrix(Rect::new(0, 0, self.size as i32, self.size as i32))
    }

    fn texture(&self) -> &GpuTexture {
        &self.framebuffer.color_attachments()[0].texture
    }
}

pub struct HighDynamicRangeRenderer {
    adaptation_chain: AdaptationChain,
    downscale_chain: [LumBuffer; 6],
    frame_luminance: LumBuffer,
    adaptation_shader: RenderPassContainer,
    luminance_shader: RenderPassContainer,
    downscale_shader: RenderPassContainer,
    map_shader: RenderPassContainer,
    stub_lut: GpuTexture,
    lum_calculation_method: LuminanceCalculationMethod,
}

impl HighDynamicRangeRenderer {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            frame_luminance: LumBuffer::new(server, 64)?,
            downscale_chain: [
                LumBuffer::new(server, 32)?,
                LumBuffer::new(server, 16)?,
                LumBuffer::new(server, 8)?,
                LumBuffer::new(server, 4)?,
                LumBuffer::new(server, 2)?,
                LumBuffer::new(server, 1)?,
            ],
            adaptation_chain: AdaptationChain::new(server)?,
            adaptation_shader: RenderPassContainer::from_str(
                server,
                include_str!("../shaders/hdr_adaptation.shader"),
            )?,
            luminance_shader: RenderPassContainer::from_str(
                server,
                include_str!("../shaders/hdr_luminance.shader"),
            )?,
            downscale_shader: RenderPassContainer::from_str(
                server,
                include_str!("../shaders/hdr_downscale.shader"),
            )?,
            map_shader: RenderPassContainer::from_str(
                server,
                include_str!("../shaders/hdr_map.shader"),
            )?,
            stub_lut: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Volume {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                pixel_kind: PixelKind::RGB8,
                data: Some(&[0, 0, 0]),
                ..Default::default()
            })?,
            lum_calculation_method: LuminanceCalculationMethod::DownSampling,
        })
    }

    fn calculate_frame_luminance(
        &self,
        scene_frame: &GpuTexture,
        quad: &GpuGeometryBuffer,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        self.frame_luminance.clear();

        let frame_matrix = self.frame_luminance.matrix();
        let inv_size = Vector2::repeat(1.0 / self.frame_luminance.size as f32);

        let properties = PropertyGroup::from([
            property("worldViewProjection", &frame_matrix),
            property("invSize", &inv_size),
        ]);
        let material = RenderMaterial::from([
            binding(
                "frameSampler",
                (scene_frame, &fallback_resources.nearest_clamp_sampler),
            ),
            binding("properties", &properties),
        ]);

        self.luminance_shader.run_pass(
            1,
            &ImmutableString::new("Primary"),
            &self.frame_luminance.framebuffer,
            quad,
            Rect::new(
                0,
                0,
                self.frame_luminance.size as i32,
                self.frame_luminance.size as i32,
            ),
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )
    }

    fn calculate_avg_frame_luminance(
        &self,
        quad: &GpuGeometryBuffer,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        match self.lum_calculation_method {
            LuminanceCalculationMethod::Histogram => {
                // TODO: Cloning memory from GPU to CPU is slow, but since the engine is limited
                // by macOS's OpenGL 4.1 support and lack of compute shaders we'll build histogram
                // manually on CPU anyway. Replace this with compute shaders whenever possible.
                let pixels = self
                    .frame_luminance
                    .framebuffer
                    .read_pixels_of_type::<f32>(ReadTarget::Color(0))
                    .ok_or_else(|| {
                        FrameworkError::Custom("Unable to read luminance buffer!".to_string())
                    })?;

                let evaluator =
                    luminance::histogram_luminance_evaluator::HistogramLuminanceEvaluator::default(
                    );
                let avg_value = evaluator.average_luminance(&pixels);

                self.downscale_chain.last().unwrap().texture().set_data(
                    GpuTextureKind::Rectangle {
                        width: 1,
                        height: 1,
                    },
                    PixelKind::R32F,
                    1,
                    Some(value_as_u8_slice(&avg_value)),
                )?;
            }
            LuminanceCalculationMethod::DownSampling => {
                let mut prev_luminance = self.frame_luminance.texture();

                for lum_buffer in self.downscale_chain.iter() {
                    let inv_size = Vector2::repeat(1.0 / lum_buffer.size as f32);
                    let matrix = lum_buffer.matrix();

                    let properties = PropertyGroup::from([
                        property("worldViewProjection", &matrix),
                        property("invSize", &inv_size),
                    ]);
                    let material = RenderMaterial::from([
                        binding(
                            "lumSampler",
                            (prev_luminance, &fallback_resources.nearest_clamp_sampler),
                        ),
                        binding("properties", &properties),
                    ]);

                    stats += self.downscale_shader.run_pass(
                        1,
                        &ImmutableString::new("Primary"),
                        &lum_buffer.framebuffer,
                        quad,
                        Rect::new(0, 0, lum_buffer.size as i32, lum_buffer.size as i32),
                        &material,
                        uniform_buffer_cache,
                        Default::default(),
                        None,
                    )?;

                    prev_luminance = lum_buffer.texture();
                }
            }
        }

        Ok(stats)
    }

    fn adaptation(
        &self,
        quad: &GpuGeometryBuffer,
        dt: f32,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let ctx = self.adaptation_chain.begin();
        let viewport = Rect::new(0, 0, ctx.lum_buffer.size as i32, ctx.lum_buffer.size as i32);
        let matrix = ctx.lum_buffer.matrix();

        let speed = 0.3 * dt;
        let properties = PropertyGroup::from([
            property("worldViewProjection", &matrix),
            property("speed", &speed),
        ]);
        let material = RenderMaterial::from([
            binding(
                "oldLumSampler",
                (&ctx.prev_lum, &fallback_resources.nearest_clamp_sampler),
            ),
            binding(
                "newLumSampler",
                (
                    self.downscale_chain.last().unwrap().texture(),
                    &fallback_resources.nearest_clamp_sampler,
                ),
            ),
            binding("properties", &properties),
        ]);

        self.adaptation_shader.run_pass(
            1,
            &ImmutableString::new("Primary"),
            &ctx.lum_buffer.framebuffer,
            quad,
            viewport,
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )
    }

    fn map_hdr_to_ldr(
        &self,
        server: &dyn GraphicsServer,
        hdr_scene_frame: &GpuTexture,
        bloom_texture: &GpuTexture,
        ldr_framebuffer: &GpuFrameBuffer,
        viewport: Rect<i32>,
        quad: &GpuGeometryBuffer,
        exposure: Exposure,
        color_grading_lut: Option<&ColorGradingLut>,
        use_color_grading: bool,
        texture_cache: &mut TextureCache,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let frame_matrix = make_viewport_matrix(viewport);

        let color_grading_lut_tex = color_grading_lut
            .and_then(|l| {
                texture_cache
                    .get(server, l.lut_ref())
                    .map(|t| (&t.gpu_texture, &t.gpu_sampler))
            })
            .unwrap_or((&self.stub_lut, &fallback_resources.nearest_clamp_sampler));

        let (is_auto, key_value, min_luminance, max_luminance, fixed_exposure) = match exposure {
            Exposure::Auto {
                key_value,
                min_luminance,
                max_luminance,
            } => (true, key_value, min_luminance, max_luminance, 0.0),
            Exposure::Manual(fixed_exposure) => (false, 0.0, 0.0, 0.0, fixed_exposure),
        };

        let color_grading_enabled = use_color_grading && color_grading_lut.is_some();
        let properties = PropertyGroup::from([
            property("worldViewProjection", &frame_matrix),
            property("useColorGrading", &color_grading_enabled),
            property("keyValue", &key_value),
            property("minLuminance", &min_luminance),
            property("maxLuminance", &max_luminance),
            property("autoExposure", &is_auto),
            property("fixedExposure", &fixed_exposure),
        ]);
        let material = RenderMaterial::from([
            binding(
                "hdrSampler",
                (hdr_scene_frame, &fallback_resources.nearest_clamp_sampler),
            ),
            binding(
                "lumSampler",
                (
                    self.adaptation_chain.avg_lum_texture(),
                    &fallback_resources.nearest_clamp_sampler,
                ),
            ),
            binding(
                "bloomSampler",
                (bloom_texture, &fallback_resources.linear_clamp_sampler),
            ),
            binding("colorMapSampler", color_grading_lut_tex),
            binding("properties", &properties),
        ]);

        self.map_shader.run_pass(
            1,
            &ImmutableString::new("Primary"),
            ldr_framebuffer,
            quad,
            viewport,
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )
    }

    pub fn render(
        &self,
        server: &dyn GraphicsServer,
        hdr_scene_frame: &GpuTexture,
        bloom_texture: &GpuTexture,
        ldr_framebuffer: &GpuFrameBuffer,
        viewport: Rect<i32>,
        quad: &GpuGeometryBuffer,
        dt: f32,
        exposure: Exposure,
        color_grading_lut: Option<&ColorGradingLut>,
        use_color_grading: bool,
        texture_cache: &mut TextureCache,
        uniform_buffer_cache: &mut UniformBufferCache,
        fallback_resources: &FallbackResources,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();
        stats += self.calculate_frame_luminance(
            hdr_scene_frame,
            quad,
            uniform_buffer_cache,
            fallback_resources,
        )?;
        stats +=
            self.calculate_avg_frame_luminance(quad, uniform_buffer_cache, fallback_resources)?;
        stats += self.adaptation(quad, dt, uniform_buffer_cache, fallback_resources)?;
        stats += self.map_hdr_to_ldr(
            server,
            hdr_scene_frame,
            bloom_texture,
            ldr_framebuffer,
            viewport,
            quad,
            exposure,
            color_grading_lut,
            use_color_grading,
            texture_cache,
            uniform_buffer_cache,
            fallback_resources,
        )?;
        Ok(stats)
    }
}
