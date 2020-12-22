use crate::{
    core::{
        algebra::{Matrix3, Matrix4, Vector2, Vector3},
        color::Color,
        math::{lerpf, Rect},
        scope_profile,
    },
    rand::Rng,
    renderer::{
        blur::Blur,
        error::RendererError,
        framework::{
            framebuffer::{
                Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer, FrameBufferTrait,
            },
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::PipelineState,
        },
        gbuffer::GBuffer,
        surface::SurfaceSharedData,
        GeometryCache, RenderPassStatistics,
    },
};
use std::{cell::RefCell, rc::Rc};

// Keep in sync with shader define.
const KERNEL_SIZE: usize = 32;

// Size of noise texture.
const NOISE_SIZE: usize = 4;

struct Shader {
    program: GpuProgram,
    depth_sampler: UniformLocation,
    normal_sampler: UniformLocation,
    noise_sampler: UniformLocation,
    radius: UniformLocation,
    kernel: UniformLocation,
    projection_matrix: UniformLocation,
    noise_scale: UniformLocation,
    inv_proj_matrix: UniformLocation,
    world_view_proj_matrix: UniformLocation,
    view_matrix: UniformLocation,
}

impl Shader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/ssao_fs.glsl");
        let vertex_source = include_str!("shaders/ssao_vs.glsl");
        let program = GpuProgram::from_source("SsaoShader", vertex_source, fragment_source)?;
        Ok(Self {
            depth_sampler: program.uniform_location("depthSampler")?,
            normal_sampler: program.uniform_location("normalSampler")?,
            noise_sampler: program.uniform_location("noiseSampler")?,
            kernel: program.uniform_location("kernel")?,
            radius: program.uniform_location("radius")?,
            projection_matrix: program.uniform_location("projectionMatrix")?,
            inv_proj_matrix: program.uniform_location("inverseProjectionMatrix")?,
            noise_scale: program.uniform_location("noiseScale")?,
            world_view_proj_matrix: program.uniform_location("worldViewProjection")?,
            view_matrix: program.uniform_location("viewMatrix")?,
            program,
        })
    }
}

pub struct ScreenSpaceAmbientOcclusionRenderer {
    blur: Blur,
    shader: Shader,
    framebuffer: FrameBuffer,
    quad: SurfaceSharedData,
    width: i32,
    height: i32,
    noise: Rc<RefCell<GpuTexture>>,
    kernel: [Vector3<f32>; KERNEL_SIZE],
    radius: f32,
}

impl ScreenSpaceAmbientOcclusionRenderer {
    pub fn new(
        state: &mut PipelineState,
        frame_width: usize,
        frame_height: usize,
    ) -> Result<Self, RendererError> {
        // It is good balance between quality and performance, no need to do SSAO in full resolution.
        // This SSAO map size reduction was taken from DOOM (2016).
        let width = (frame_width / 2).max(1);
        let height = (frame_height / 2).max(1);

        let occlusion = {
            let kind = GpuTextureKind::Rectangle { width, height };
            let mut texture = GpuTexture::new(
                state,
                kind,
                PixelKind::F32,
                MinificationFilter::Nearest,
                MagnificationFilter::Nearest,
                1,
                None,
            )?;
            texture
                .bind_mut(state, 0)
                .set_minification_filter(MinificationFilter::Nearest)
                .set_magnification_filter(MagnificationFilter::Nearest);
            texture
        };

        let mut rng = crate::rand::thread_rng();

        Ok(Self {
            blur: Blur::new(state, width, height)?,
            shader: Shader::new()?,
            framebuffer: FrameBuffer::new(
                state,
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(occlusion)),
                }],
            )?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
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
                    .try_normalize(std::f32::EPSILON)
                    .unwrap()
                    // Use non-uniform distribution to shuffle points inside hemisphere.
                    .scale(scale * rng.gen_range(0.0..1.0));
                }
                kernel
            },
            noise: Rc::new(RefCell::new({
                const RGB_PIXEL_SIZE: usize = 3;
                let mut pixels = [0u8; RGB_PIXEL_SIZE * NOISE_SIZE * NOISE_SIZE];
                for pixel in pixels.chunks_exact_mut(RGB_PIXEL_SIZE) {
                    pixel[0] = rng.gen_range(0u8..255u8); // R
                    pixel[1] = rng.gen_range(0u8..255u8); // G
                    pixel[2] = 0u8; // B
                }
                let kind = GpuTextureKind::Rectangle {
                    width: NOISE_SIZE,
                    height: NOISE_SIZE,
                };
                let mut texture = GpuTexture::new(
                    state,
                    kind,
                    PixelKind::RGB8,
                    MinificationFilter::Nearest,
                    MagnificationFilter::Nearest,
                    1,
                    Some(&pixels),
                )?;
                texture
                    .bind_mut(state, 0)
                    .set_wrap(Coordinate::S, WrapMode::Repeat)
                    .set_wrap(Coordinate::T, WrapMode::Repeat);
                texture
            })),
            radius: 0.5,
        })
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.abs();
    }

    fn raw_ao_map(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn ao_map(&self) -> Rc<RefCell<GpuTexture>> {
        self.blur.result()
    }

    pub(in crate) fn render(
        &mut self,
        state: &mut PipelineState,
        gbuffer: &GBuffer,
        geom_cache: &mut GeometryCache,
        projection_matrix: Matrix4<f32>,
        view_matrix: Matrix3<f32>,
    ) -> RenderPassStatistics {
        scope_profile!();

        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.width, self.height);

        let frame_matrix = Matrix4::new_orthographic(
            0.0,
            viewport.w() as f32,
            viewport.h() as f32,
            0.0,
            -1.0,
            1.0,
        ) * Matrix4::new_nonuniform_scaling(&Vector3::new(
            viewport.w() as f32,
            viewport.h() as f32,
            0.0,
        ));

        self.framebuffer.clear(
            state,
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            None,
        );

        stats += self.framebuffer.draw(
            geom_cache.get(state, &self.quad),
            state,
            viewport,
            &self.shader.program,
            &DrawParameters {
                cull_face: CullFace::Back,
                culling: false,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: false,
                depth_test: false,
                blend: false,
            },
            &[
                (
                    self.shader.depth_sampler,
                    UniformValue::Sampler {
                        index: 0,
                        texture: gbuffer.depth(),
                    },
                ),
                (
                    self.shader.normal_sampler,
                    UniformValue::Sampler {
                        index: 1,
                        texture: gbuffer.normal_texture(),
                    },
                ),
                (
                    self.shader.noise_sampler,
                    UniformValue::Sampler {
                        index: 2,
                        texture: self.noise.clone(),
                    },
                ),
                (self.shader.kernel, UniformValue::Vec3Array(&self.kernel)),
                (self.shader.radius, UniformValue::Float(self.radius)),
                (
                    self.shader.noise_scale,
                    UniformValue::Vector2({
                        Vector2::new(
                            self.width as f32 / NOISE_SIZE as f32,
                            self.height as f32 / NOISE_SIZE as f32,
                        )
                    }),
                ),
                (
                    self.shader.world_view_proj_matrix,
                    UniformValue::Matrix4(frame_matrix),
                ),
                (
                    self.shader.projection_matrix,
                    UniformValue::Matrix4(projection_matrix),
                ),
                (
                    self.shader.inv_proj_matrix,
                    UniformValue::Matrix4(projection_matrix.try_inverse().unwrap_or_default()),
                ),
                (self.shader.view_matrix, UniformValue::Matrix3(view_matrix)),
            ],
        );

        self.blur.render(state, geom_cache, self.raw_ao_map());

        stats
    }
}
