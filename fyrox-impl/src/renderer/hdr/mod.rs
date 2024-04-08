use crate::renderer::framework::geometry_buffer::ElementRange;
use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        color::Color,
        math::Rect,
    },
    renderer::{
        cache::texture::TextureCache,
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, DrawParameters, FrameBuffer},
            geometry_buffer::{DrawCallStatistics, GeometryBuffer},
            gpu_texture::{
                GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
            },
            state::PipelineState,
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
use fyrox_core::{transmute_slice, value_as_u8_slice};
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
    framebuffer: FrameBuffer,
    size: usize,
}

impl LumBuffer {
    fn new(state: &PipelineState, size: usize) -> Result<Self, FrameworkError> {
        let texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle {
                width: size,
                height: size,
            },
            PixelKind::R32F,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        Ok(Self {
            framebuffer: FrameBuffer::new(
                state,
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(texture)),
                }],
            )?,
            size,
        })
    }

    fn clear(&mut self, state: &PipelineState) {
        self.framebuffer.clear(
            state,
            Rect::new(0, 0, self.size as i32, self.size as i32),
            Some(Color::BLACK),
            None,
            None,
        );
    }

    fn matrix(&self) -> Matrix4<f32> {
        Matrix4::new_orthographic(0.0, self.size as f32, self.size as f32, 0.0, -1.0, 1.0)
            * Matrix4::new_nonuniform_scaling(&Vector3::new(
                self.size as f32,
                self.size as f32,
                0.0,
            ))
    }

    fn texture(&self) -> Rc<RefCell<GpuTexture>> {
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
    stub_lut: Rc<RefCell<GpuTexture>>,
    lum_calculation_method: LuminanceCalculationMethod,
}

impl HighDynamicRangeRenderer {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            frame_luminance: LumBuffer::new(state, 64)?,
            downscale_chain: [
                LumBuffer::new(state, 32)?,
                LumBuffer::new(state, 16)?,
                LumBuffer::new(state, 8)?,
                LumBuffer::new(state, 4)?,
                LumBuffer::new(state, 2)?,
                LumBuffer::new(state, 1)?,
            ],
            adaptation_chain: AdaptationChain::new(state)?,
            adaptation_shader: AdaptationShader::new(state)?,
            luminance_shader: LuminanceShader::new(state)?,
            downscale_shader: DownscaleShader::new(state)?,
            map_shader: MapShader::new(state)?,
            stub_lut: Rc::new(RefCell::new(GpuTexture::new(
                state,
                GpuTextureKind::Volume {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                PixelKind::RGB8,
                MinificationFilter::Linear,
                MagnificationFilter::Linear,
                1,
                Some(&[0, 0, 0]),
            )?)),
            lum_calculation_method: LuminanceCalculationMethod::DownSampling,
        })
    }

    fn calculate_frame_luminance(
        &mut self,
        state: &PipelineState,
        scene_frame: Rc<RefCell<GpuTexture>>,
        quad: &GeometryBuffer,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        self.frame_luminance.clear(state);
        let frame_matrix = self.frame_luminance.matrix();

        let shader = &self.luminance_shader;
        let inv_size = 1.0 / self.frame_luminance.size as f32;
        self.frame_luminance.framebuffer.draw(
            quad,
            state,
            Rect::new(
                0,
                0,
                self.frame_luminance.size as i32,
                self.frame_luminance.size as i32,
            ),
            &shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: false,
                blend: None,
                stencil_op: Default::default(),
            },
            ElementRange::Full,
            |mut program_binding| {
                program_binding
                    .set_matrix4(&shader.wvp_matrix, &frame_matrix)
                    .set_vector2(&shader.inv_size, &Vector2::new(inv_size, inv_size))
                    .set_texture(&shader.frame_sampler, &scene_frame);
            },
        )
    }

    fn calculate_avg_frame_luminance(
        &mut self,
        state: &PipelineState,
        quad: &GeometryBuffer,
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
                let data = self
                    .frame_luminance
                    .texture()
                    .borrow_mut()
                    .bind_mut(state, 0)
                    .read_pixels(state);

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
                    .bind_mut(state, 0)
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
                        state,
                        Rect::new(0, 0, lum_buffer.size as i32, lum_buffer.size as i32),
                        &shader.program,
                        &DrawParameters {
                            cull_face: None,
                            color_write: Default::default(),
                            depth_write: false,
                            stencil_test: None,
                            depth_test: false,
                            blend: None,
                            stencil_op: Default::default(),
                        },
                        ElementRange::Full,
                        |mut program_binding| {
                            program_binding
                                .set_matrix4(&shader.wvp_matrix, &matrix)
                                .set_vector2(&shader.inv_size, &Vector2::new(inv_size, inv_size))
                                .set_texture(&shader.lum_sampler, &prev_luminance);
                        },
                    )?;

                    prev_luminance = lum_buffer.texture();
                }
            }
        }

        Ok(stats)
    }

    fn adaptation(
        &mut self,
        state: &PipelineState,
        quad: &GeometryBuffer,
        dt: f32,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let new_lum = self.downscale_chain.last().unwrap().texture();
        let ctx = self.adaptation_chain.begin();
        let viewport = Rect::new(0, 0, ctx.lum_buffer.size as i32, ctx.lum_buffer.size as i32);
        let shader = &self.adaptation_shader;
        let matrix = ctx.lum_buffer.matrix();
        let prev_lum = ctx.prev_lum;
        ctx.lum_buffer.framebuffer.draw(
            quad,
            state,
            viewport,
            &shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: false,
                blend: None,
                stencil_op: Default::default(),
            },
            ElementRange::Full,
            |mut program_binding| {
                program_binding
                    .set_matrix4(&shader.wvp_matrix, &matrix)
                    .set_texture(&shader.old_lum_sampler, &prev_lum)
                    .set_texture(&shader.new_lum_sampler, &new_lum)
                    .set_f32(&shader.speed, 0.3 * dt) // TODO: Make configurable
                ;
            },
        )
    }

    fn map_hdr_to_ldr(
        &mut self,
        state: &PipelineState,
        hdr_scene_frame: Rc<RefCell<GpuTexture>>,
        bloom_texture: Rc<RefCell<GpuTexture>>,
        ldr_framebuffer: &mut FrameBuffer,
        viewport: Rect<i32>,
        quad: &GeometryBuffer,
        exposure: Exposure,
        color_grading_lut: Option<&ColorGradingLut>,
        use_color_grading: bool,
        texture_cache: &mut TextureCache,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let shader = &self.map_shader;
        let frame_matrix = make_viewport_matrix(viewport);
        let avg_lum = self.adaptation_chain.avg_lum_texture();

        let color_grading_lut_tex = color_grading_lut
            .and_then(|l| texture_cache.get(state, l.lut_ref()))
            .unwrap_or(&self.stub_lut);

        ldr_framebuffer.draw(
            quad,
            state,
            viewport,
            &shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: false,
                blend: None,
                stencil_op: Default::default(),
            },
            ElementRange::Full,
            |mut program_binding| {
                let program_binding = program_binding
                    .set_matrix4(&shader.wvp_matrix, &frame_matrix)
                    .set_texture(&shader.lum_sampler, &avg_lum)
                    .set_texture(&shader.bloom_sampler, &bloom_texture)
                    .set_texture(&shader.hdr_sampler, &hdr_scene_frame)
                    .set_bool(
                        &shader.use_color_grading,
                        use_color_grading && color_grading_lut.is_some(),
                    )
                    .set_texture(&shader.color_map_sampler, color_grading_lut_tex);

                match exposure {
                    Exposure::Auto {
                        key_value,
                        min_luminance,
                        max_luminance,
                    } => {
                        program_binding
                            .set_bool(&shader.auto_exposure, true)
                            .set_f32(&shader.key_value, key_value)
                            .set_f32(&shader.min_luminance, min_luminance)
                            .set_f32(&shader.max_luminance, max_luminance);
                    }
                    Exposure::Manual(fixed_exposure) => {
                        program_binding
                            .set_bool(&shader.auto_exposure, false)
                            .set_f32(&shader.fixed_exposure, fixed_exposure);
                    }
                }
            },
        )
    }

    pub fn render(
        &mut self,
        state: &PipelineState,
        hdr_scene_frame: Rc<RefCell<GpuTexture>>,
        bloom_texture: Rc<RefCell<GpuTexture>>,
        ldr_framebuffer: &mut FrameBuffer,
        viewport: Rect<i32>,
        quad: &GeometryBuffer,
        dt: f32,
        exposure: Exposure,
        color_grading_lut: Option<&ColorGradingLut>,
        use_color_grading: bool,
        texture_cache: &mut TextureCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();
        stats += self.calculate_frame_luminance(state, hdr_scene_frame.clone(), quad)?;
        stats += self.calculate_avg_frame_luminance(state, quad)?;
        stats += self.adaptation(state, quad, dt)?;
        stats += self.map_hdr_to_ldr(
            state,
            hdr_scene_frame,
            bloom_texture,
            ldr_framebuffer,
            viewport,
            quad,
            exposure,
            color_grading_lut,
            use_color_grading,
            texture_cache,
        )?;
        Ok(stats)
    }
}
