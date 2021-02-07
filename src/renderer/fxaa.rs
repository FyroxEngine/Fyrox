use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        math::Rect,
    },
    renderer::{
        error::RendererError,
        framework::{
            framebuffer::{CullFace, DrawParameters, FrameBufferTrait},
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::GpuTexture,
            state::PipelineState,
        },
        surface::SurfaceSharedData,
        GeometryCache, RenderPassStatistics,
    },
};
use std::{cell::RefCell, rc::Rc};

struct FxaaShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub screen_texture: UniformLocation,
    pub inverse_screen_size: UniformLocation,
}

impl FxaaShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/fxaa_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");

        let program = GpuProgram::from_source("FXAAShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            screen_texture: program.uniform_location("screenTexture")?,
            inverse_screen_size: program.uniform_location("inverseScreenSize")?,
            program,
        })
    }
}

pub struct FxaaRenderer {
    shader: FxaaShader,
    quad: SurfaceSharedData,
}

impl FxaaRenderer {
    pub fn new() -> Result<Self, RendererError> {
        Ok(Self {
            shader: FxaaShader::new()?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
        })
    }

    pub(in crate) fn render(
        &self,
        state: &mut PipelineState,
        viewport: Rect<i32>,
        frame_texture: Rc<RefCell<GpuTexture>>,
        frame_buffer: &mut dyn FrameBufferTrait,
        geom_cache: &mut GeometryCache,
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        let quad = geom_cache.get(state, &self.quad);

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

        statistics += frame_buffer.draw(
            quad,
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
                (self.shader.wvp_matrix, UniformValue::Matrix4(frame_matrix)),
                (
                    self.shader.inverse_screen_size,
                    UniformValue::Vector2(Vector2::new(
                        1.0 / viewport.w() as f32,
                        1.0 / viewport.h() as f32,
                    )),
                ),
                (
                    self.shader.screen_texture,
                    UniformValue::Sampler {
                        index: 0,
                        texture: frame_texture,
                    },
                ),
            ],
        );

        statistics
    }
}
