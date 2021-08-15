use crate::renderer::framework::framebuffer::FrameBuffer;
use crate::{
    core::{
        algebra::{Matrix4, Vector3},
        math::Rect,
    },
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{CullFace, DrawParameters},
        gpu_program::{GpuProgram, UniformLocation},
        gpu_texture::GpuTexture,
        state::PipelineState,
    },
    renderer::{GeometryCache, RenderPassStatistics},
    scene::mesh::surface::SurfaceData,
};
use std::{cell::RefCell, rc::Rc};

struct Shader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub screen_texture: UniformLocation,
}

impl Shader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/gamma_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");

        let program =
            GpuProgram::from_source(state, "GammaShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            screen_texture: program.uniform_location(state, "screenTexture")?,
            program,
        })
    }
}

pub struct GammaCorrectionPass {
    shader: Shader,
    quad: SurfaceData,
}

impl GammaCorrectionPass {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            shader: Shader::new(state)?,
            quad: SurfaceData::make_unit_xy_quad(),
        })
    }

    pub(in crate) fn render(
        &self,
        state: &mut PipelineState,
        viewport: Rect<i32>,
        frame_texture: Rc<RefCell<GpuTexture>>,
        frame_buffer: &mut FrameBuffer,
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
            |program_binding| {
                program_binding
                    .set_matrix4(&self.shader.wvp_matrix, &frame_matrix)
                    .set_texture(&self.shader.screen_texture, &frame_texture);
            },
        );

        statistics
    }
}
