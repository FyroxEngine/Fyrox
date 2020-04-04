use std::{
    cell::RefCell,
    rc::Rc,
};
use rand::Rng;
use crate::{
    renderer::{
        surface::SurfaceSharedData,
        gbuffer::GBuffer,
        GeometryCache,
        error::RendererError,
        RenderPassStatistics,
        framework::{
            framebuffer::{
                DrawParameters,
                CullFace,
                FrameBuffer,
                Attachment,
                AttachmentKind,
                FrameBufferTrait,
            },
            state::State,
            gpu_texture::{
                GpuTexture,
                GpuTextureKind,
                PixelKind,
                MininificationFilter,
                MagnificationFilter,
                Coordinate,
                WrapMode,
            },
            gpu_program::{
                GpuProgram,
                UniformLocation,
                UniformValue,
            },
        },
        blur::Blur
    },
    core::{
        scope_profile,
        math::{
            vec3::Vec3,
            Rect,
            mat3::Mat3,
            mat4::Mat4,
            vec2::Vec2,
            lerpf,
        },
        color::Color,
    },
};

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
    kernel: [Vec3; KERNEL_SIZE],
    radius: f32
}

impl ScreenSpaceAmbientOcclusionRenderer {
    pub fn new(state: &mut State, width: usize, height: usize) -> Result<Self, RendererError> {
        let occlusion = {
            let kind = GpuTextureKind::Rectangle { width, height };
            let mut texture = GpuTexture::new(state, kind, PixelKind::F32, None)?;
            texture.bind_mut(state, 0)
                .set_minification_filter(MininificationFilter::Nearest)
                .set_magnification_filter(MagnificationFilter::Nearest);
            texture
        };

        let mut rng = rand::thread_rng();

        Ok(Self {
            blur: Blur::new(state, width, height)?,
            shader: Shader::new()?,
            framebuffer: FrameBuffer::new(
                state,
                None,
                vec![
                    Attachment {
                        kind: AttachmentKind::Color,
                        texture: Rc::new(RefCell::new(occlusion)),
                    },
                ])?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
            width: width as i32,
            height: height as i32,
            kernel: {
                let mut kernel = [Default::default(); KERNEL_SIZE];
                for (i, v) in kernel.iter_mut().enumerate() {
                    let k = i as f32 / KERNEL_SIZE as f32;
                    let scale = lerpf(0.1, 1.0, k * k);
                    *v = Vec3::new(
                        rng.gen_range(-1.0, 1.0),
                        rng.gen_range(-1.0, 1.0),
                        rng.gen_range(0.0, 1.0))
                        // Make sphere
                        .normalized()
                        .unwrap()
                        // Use non-uniform distribution to shuffle points inside hemisphere.
                        .scale(scale * rng.gen_range(0.0, 1.0));
                }
                kernel
            },
            noise: Rc::new(RefCell::new({
                const RGB_PIXEL_SIZE: usize = 3;
                let mut pixels = [0; RGB_PIXEL_SIZE * NOISE_SIZE * NOISE_SIZE];
                for pixel in pixels.chunks_exact_mut(RGB_PIXEL_SIZE) {
                    pixel[0] = rng.gen_range(0, 255); // R
                    pixel[1] = rng.gen_range(0, 255); // G
                    pixel[2] = 0; // B
                }
                let kind = GpuTextureKind::Rectangle { width: NOISE_SIZE, height: NOISE_SIZE };
                let mut texture = GpuTexture::new(state, kind, PixelKind::RGB8, Some(&pixels))?;
                texture.bind_mut(state, 0)
                    .set_wrap(Coordinate::S, WrapMode::Repeat)
                    .set_wrap(Coordinate::T, WrapMode::Repeat);
                texture
            })),
            radius: 0.5
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

    pub fn render(&mut self,
                  state: &mut State,
                  gbuffer: &GBuffer,
                  geom_cache: &mut GeometryCache,
                  projection_matrix: Mat4,
                  view_matrix: Mat3,
    ) -> RenderPassStatistics {
        scope_profile!();

        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.width, self.height);

        let frame_matrix =
            Mat4::ortho(0.0, viewport.w as f32, viewport.h as f32, 0.0, -1.0, 1.0) *
                Mat4::scale(Vec3::new(viewport.w as f32, viewport.h as f32, 0.0));

        self.framebuffer.clear(state, viewport, Some(Color::from_rgba(0, 0, 0, 0)), Some(1.0), None);

        stats.add_draw_call(
            self.framebuffer.draw(
                state,
                viewport,
                geom_cache.get(&self.quad),
                &mut self.shader.program,
                DrawParameters {
                    cull_face: CullFace::Back,
                    culling: false,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: false,
                    depth_test: false,
                    blend: false,
                },
                &[
                    (self.shader.depth_sampler, UniformValue::Sampler { index: 0, texture: gbuffer.depth() }),
                    (self.shader.normal_sampler, UniformValue::Sampler { index: 1, texture: gbuffer.normal_texture() }),
                    (self.shader.noise_sampler, UniformValue::Sampler { index: 2, texture: self.noise.clone() }),
                    (self.shader.kernel, UniformValue::Vec3Array(&self.kernel)),
                    (self.shader.radius, UniformValue::Float(self.radius)),
                    (self.shader.noise_scale, UniformValue::Vec2({
                        Vec2::new(self.width as f32 / NOISE_SIZE as f32,
                                  self.height as f32 / NOISE_SIZE as f32)
                    })),
                    (self.shader.world_view_proj_matrix, UniformValue::Mat4(frame_matrix)),
                    (self.shader.projection_matrix, UniformValue::Mat4(projection_matrix)),
                    (self.shader.inv_proj_matrix, UniformValue::Mat4(projection_matrix.inverse().unwrap())),
                    (self.shader.view_matrix, UniformValue::Mat3(view_matrix))
                ],
            )
        );

        self.blur.render(state, geom_cache, self.raw_ao_map());

        stats
    }
}