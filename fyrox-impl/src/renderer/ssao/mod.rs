use crate::renderer::framework::geometry_buffer::ElementRange;
use crate::{
    core::{
        algebra::{Matrix3, Matrix4, Vector2, Vector3},
        color::Color,
        math::{lerpf, Rect},
        scope_profile,
        sstorage::ImmutableString,
    },
    rand::Rng,
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, DrawParameters, FrameBuffer},
            geometry_buffer::{GeometryBuffer, GeometryBufferKind},
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::PipelineState,
        },
        gbuffer::GBuffer,
        ssao::blur::Blur,
        RenderPassStatistics,
    },
    scene::mesh::surface::SurfaceData,
};
use std::{cell::RefCell, rc::Rc};

mod blur;

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
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/ssao_fs.glsl");
        let vertex_source = include_str!("../shaders/ssao_vs.glsl");
        let program = GpuProgram::from_source(state, "SsaoShader", vertex_source, fragment_source)?;
        Ok(Self {
            depth_sampler: program
                .uniform_location(state, &ImmutableString::new("depthSampler"))?,
            normal_sampler: program
                .uniform_location(state, &ImmutableString::new("normalSampler"))?,
            noise_sampler: program
                .uniform_location(state, &ImmutableString::new("noiseSampler"))?,
            kernel: program.uniform_location(state, &ImmutableString::new("kernel"))?,
            radius: program.uniform_location(state, &ImmutableString::new("radius"))?,
            projection_matrix: program
                .uniform_location(state, &ImmutableString::new("projectionMatrix"))?,
            inv_proj_matrix: program
                .uniform_location(state, &ImmutableString::new("inverseProjectionMatrix"))?,
            noise_scale: program.uniform_location(state, &ImmutableString::new("noiseScale"))?,
            world_view_proj_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            view_matrix: program.uniform_location(state, &ImmutableString::new("viewMatrix"))?,
            program,
        })
    }
}

pub struct ScreenSpaceAmbientOcclusionRenderer {
    blur: Blur,
    shader: Shader,
    framebuffer: FrameBuffer,
    quad: GeometryBuffer,
    width: i32,
    height: i32,
    noise: Rc<RefCell<GpuTexture>>,
    kernel: [Vector3<f32>; KERNEL_SIZE],
    radius: f32,
}

impl ScreenSpaceAmbientOcclusionRenderer {
    pub fn new(
        state: &PipelineState,
        frame_width: usize,
        frame_height: usize,
    ) -> Result<Self, FrameworkError> {
        // It is good balance between quality and performance, no need to do SSAO in full resolution.
        // This SSAO map size reduction was taken from DOOM (2016).
        let width = (frame_width / 2).max(1);
        let height = (frame_height / 2).max(1);

        let occlusion = {
            let kind = GpuTextureKind::Rectangle { width, height };
            let mut texture = GpuTexture::new(
                state,
                kind,
                PixelKind::R32F,
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
            shader: Shader::new(state)?,
            framebuffer: FrameBuffer::new(
                state,
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(occlusion)),
                }],
            )?,
            quad: GeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                GeometryBufferKind::StaticDraw,
                state,
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

    pub(crate) fn render(
        &mut self,
        state: &PipelineState,
        gbuffer: &GBuffer,
        projection_matrix: Matrix4<f32>,
        view_matrix: Matrix3<f32>,
    ) -> Result<RenderPassStatistics, FrameworkError> {
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

        let shader = &self.shader;
        let noise = &self.noise;
        let kernel = &self.kernel;
        let noise_scale = Vector2::new(
            self.width as f32 / NOISE_SIZE as f32,
            self.height as f32 / NOISE_SIZE as f32,
        );
        let radius = self.radius;
        stats += self.framebuffer.draw(
            &self.quad,
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
                    .set_texture(&shader.depth_sampler, &gbuffer.depth())
                    .set_texture(&shader.normal_sampler, &gbuffer.normal_texture())
                    .set_texture(&shader.noise_sampler, noise)
                    .set_vector3_slice(&shader.kernel, kernel)
                    .set_vector2(&shader.noise_scale, &noise_scale)
                    .set_f32(&shader.radius, radius)
                    .set_matrix4(&shader.world_view_proj_matrix, &frame_matrix)
                    .set_matrix4(&shader.projection_matrix, &projection_matrix)
                    .set_matrix4(
                        &shader.inv_proj_matrix,
                        &projection_matrix.try_inverse().unwrap_or_default(),
                    )
                    .set_matrix3(&shader.view_matrix, &view_matrix);
            },
        )?;

        self.blur.render(state, self.raw_ao_map())?;

        Ok(stats)
    }
}
