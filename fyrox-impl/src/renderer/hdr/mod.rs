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
    core::{
        algebra::{Matrix4, Vector2},
        color::Color,
        math::Rect,
        transmute_slice, value_as_u8_slice,
    },
    renderer::{
        cache::{texture::TextureCache, uniform::UniformBufferCache},
        framework::{
            error::FrameworkError,
            framebuffer::{
                Attachment, AttachmentKind, BufferLocation, FrameBuffer, ResourceBindGroup,
                ResourceBinding,
            },
            geometry_buffer::{DrawCallStatistics, GeometryBuffer},
            gpu_texture::{GpuTexture, GpuTextureDescriptor, GpuTextureKind, PixelKind},
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            DrawParameters, ElementRange,
        },
        hdr::{
            adaptation::{AdaptationChain, AdaptationShader},
            downscale::DownscaleShader,
            luminance::LuminanceShader,
            map::MapShader,
        },
        make_viewport_matrix, RenderPassStatistics,
    },
    scene::camera::{ColorGradingLut, Exposure},
};
use std::{cell::RefCell, rc::Rc};

mod adaptation;
mod downscale;
mod luminance;
mod map;

#[allow(dead_code)] // TODO
pub enum LuminanceCalculationMethod {
    Histogram,
    DownSampling,
}

pub struct LumBuffer {
    framebuffer: Box<dyn FrameBuffer>,
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

    fn clear(&mut self) {
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

    fn texture(&self) -> Rc<RefCell<dyn GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }
}

pub struct HighDynamicRangeRenderer {
    adaptation_chain: AdaptationChain,
    downscale_chain: [LumBuffer; 6],
    frame_luminance: LumBuffer,
    adaptation_shader: AdaptationShader,
    luminance_shader: LuminanceShader,
    downscale_shader: DownscaleShader,
    map_shader: MapShader,
    stub_lut: Rc<RefCell<dyn GpuTexture>>,
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
            adaptation_shader: AdaptationShader::new(server)?,
            luminance_shader: LuminanceShader::new(server)?,
            downscale_shader: DownscaleShader::new(server)?,
            map_shader: MapShader::new(server)?,
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
        &mut self,
        scene_frame: Rc<RefCell<dyn GpuTexture>>,
        quad: &dyn GeometryBuffer,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        self.frame_luminance.clear();
        let frame_matrix = self.frame_luminance.matrix();

        let shader = &self.luminance_shader;
        let inv_size = 1.0 / self.frame_luminance.size as f32;
        self.frame_luminance.framebuffer.draw(
            quad,
            Rect::new(
                0,
                0,
                self.frame_luminance.size as i32,
                self.frame_luminance.size as i32,
            ),
            &*shader.program,
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
                    ResourceBinding::texture(&scene_frame, &shader.frame_sampler),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer_cache.write(
                            StaticUniformBuffer::<256>::new()
                                .with(&frame_matrix)
                                .with(&Vector2::new(inv_size, inv_size)),
                        )?,
                        binding: BufferLocation::Auto {
                            shader_location: shader.uniform_buffer_binding,
                        },
                        data_usage: Default::default(),
                    },
                ],
            }],
            ElementRange::Full,
        )
    }

    fn calculate_avg_frame_luminance(
        &mut self,
        quad: &dyn GeometryBuffer,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        match self.lum_calculation_method {
            LuminanceCalculationMethod::Histogram => {
                let luminance_range = 0.00778f32..8.0f32;
                let log2_luminance_range = luminance_range.start.log2()..luminance_range.end.log2();
                let log2_lum_range = luminance_range.end.log2() - luminance_range.start.log2();

                // TODO: Cloning memory from GPU to CPU is slow, but since the engine is limited
                // by macOS's OpenGL 4.1 support and lack of compute shaders we'll build histogram
                // manually on CPU anyway. Replace this with compute shaders whenever possible.
                let data = self.frame_luminance.texture().borrow_mut().read_pixels();

                let pixels = transmute_slice::<u8, f32>(&data);

                // Build histogram.
                let mut bins = [0usize; 64];
                for &luminance in pixels {
                    let k = (luminance.log2() - log2_luminance_range.start) / log2_lum_range;
                    let index =
                        ((bins.len() as f32 * k) as usize).clamp(0, bins.len().saturating_sub(1));
                    bins[index] += 1;
                }

                // Compute mean value.
                let mut total_luminance = 0.0;
                let mut counter = 0;
                for (bin_index, count) in bins.iter().cloned().enumerate() {
                    let avg_luminance = log2_luminance_range.start
                        + (bin_index + 1) as f32 / bins.len() as f32 * log2_lum_range;
                    total_luminance += avg_luminance * (count as f32);
                    counter += count;
                }

                let weighted_lum = (total_luminance / counter as f32).exp2();
                let avg_lum = luminance_range.start
                    + weighted_lum * (luminance_range.end - luminance_range.start);

                self.downscale_chain
                    .last()
                    .unwrap()
                    .texture()
                    .borrow_mut()
                    .set_data(
                        GpuTextureKind::Rectangle {
                            width: 1,
                            height: 1,
                        },
                        PixelKind::R32F,
                        1,
                        Some(value_as_u8_slice(&avg_lum)),
                    )?;
            }
            LuminanceCalculationMethod::DownSampling => {
                let shader = &self.downscale_shader;
                let mut prev_luminance = self.frame_luminance.texture();
                for lum_buffer in self.downscale_chain.iter_mut() {
                    let inv_size = 1.0 / lum_buffer.size as f32;
                    let matrix = lum_buffer.matrix();
                    stats += lum_buffer.framebuffer.draw(
                        quad,
                        Rect::new(0, 0, lum_buffer.size as i32, lum_buffer.size as i32),
                        &*shader.program,
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
                                ResourceBinding::texture(&prev_luminance, &shader.lum_sampler),
                                ResourceBinding::Buffer {
                                    buffer: uniform_buffer_cache.write(
                                        StaticUniformBuffer::<256>::new()
                                            .with(&matrix)
                                            .with(&Vector2::new(inv_size, inv_size)),
                                    )?,
                                    binding: BufferLocation::Auto {
                                        shader_location: shader.uniform_buffer_binding,
                                    },
                                    data_usage: Default::default(),
                                },
                            ],
                        }],
                        ElementRange::Full,
                    )?;

                    prev_luminance = lum_buffer.texture();
                }
            }
        }

        Ok(stats)
    }

    fn adaptation(
        &mut self,
        quad: &dyn GeometryBuffer,
        dt: f32,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let ctx = self.adaptation_chain.begin();
        let viewport = Rect::new(0, 0, ctx.lum_buffer.size as i32, ctx.lum_buffer.size as i32);
        let shader = &self.adaptation_shader;
        let matrix = ctx.lum_buffer.matrix();
        ctx.lum_buffer.framebuffer.draw(
            quad,
            viewport,
            &*shader.program,
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
                    ResourceBinding::texture(&ctx.prev_lum, &shader.old_lum_sampler),
                    ResourceBinding::texture(
                        &self.downscale_chain.last().unwrap().texture(),
                        &shader.new_lum_sampler,
                    ),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer_cache.write(
                            StaticUniformBuffer::<256>::new()
                                .with(&matrix)
                                // TODO: Make configurable
                                .with(&(0.3 * dt)),
                        )?,
                        binding: BufferLocation::Auto {
                            shader_location: shader.uniform_buffer_binding,
                        },
                        data_usage: Default::default(),
                    },
                ],
            }],
            ElementRange::Full,
        )
    }

    fn map_hdr_to_ldr(
        &mut self,
        server: &dyn GraphicsServer,
        hdr_scene_frame: Rc<RefCell<dyn GpuTexture>>,
        bloom_texture: Rc<RefCell<dyn GpuTexture>>,
        ldr_framebuffer: &mut dyn FrameBuffer,
        viewport: Rect<i32>,
        quad: &dyn GeometryBuffer,
        exposure: Exposure,
        color_grading_lut: Option<&ColorGradingLut>,
        use_color_grading: bool,
        texture_cache: &mut TextureCache,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let shader = &self.map_shader;
        let frame_matrix = make_viewport_matrix(viewport);

        let color_grading_lut_tex = color_grading_lut
            .and_then(|l| texture_cache.get(server, l.lut_ref()))
            .unwrap_or(&self.stub_lut);

        let (is_auto, key_value, min_luminance, max_luminance, fixed_exposure) = match exposure {
            Exposure::Auto {
                key_value,
                min_luminance,
                max_luminance,
            } => (true, key_value, min_luminance, max_luminance, 0.0),
            Exposure::Manual(fixed_exposure) => (false, 0.0, 0.0, 0.0, fixed_exposure),
        };

        let uniform_buffer = uniform_buffer_cache.write(
            StaticUniformBuffer::<256>::new()
                .with(&frame_matrix)
                .with(&(use_color_grading && color_grading_lut.is_some()))
                .with(&key_value)
                .with(&min_luminance)
                .with(&max_luminance)
                .with(&is_auto)
                .with(&fixed_exposure),
        )?;

        ldr_framebuffer.draw(
            quad,
            viewport,
            &*shader.program,
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
                    ResourceBinding::texture(
                        &self.adaptation_chain.avg_lum_texture(),
                        &shader.lum_sampler,
                    ),
                    ResourceBinding::texture(&bloom_texture, &shader.bloom_sampler),
                    ResourceBinding::texture(&hdr_scene_frame, &shader.hdr_sampler),
                    ResourceBinding::texture(color_grading_lut_tex, &shader.color_map_sampler),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer,
                        binding: BufferLocation::Auto {
                            shader_location: shader.uniform_buffer_binding,
                        },
                        data_usage: Default::default(),
                    },
                ],
            }],
            ElementRange::Full,
        )
    }

    pub fn render(
        &mut self,
        server: &dyn GraphicsServer,
        hdr_scene_frame: Rc<RefCell<dyn GpuTexture>>,
        bloom_texture: Rc<RefCell<dyn GpuTexture>>,
        ldr_framebuffer: &mut dyn FrameBuffer,
        viewport: Rect<i32>,
        quad: &dyn GeometryBuffer,
        dt: f32,
        exposure: Exposure,
        color_grading_lut: Option<&ColorGradingLut>,
        use_color_grading: bool,
        texture_cache: &mut TextureCache,
        uniform_buffer_cache: &mut UniformBufferCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();
        stats +=
            self.calculate_frame_luminance(hdr_scene_frame.clone(), quad, uniform_buffer_cache)?;
        stats += self.calculate_avg_frame_luminance(quad, uniform_buffer_cache)?;
        stats += self.adaptation(quad, dt, uniform_buffer_cache)?;
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
        )?;
        Ok(stats)
    }
}
