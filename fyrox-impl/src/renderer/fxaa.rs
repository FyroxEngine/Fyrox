use crate::renderer::framework::geometry_buffer::ElementRange;
use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3},
        math::Rect,
        sstorage::ImmutableString,
    },
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::{DrawParameters, FrameBuffer},
            geometry_buffer::{GeometryBuffer, GeometryBufferKind},
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::GpuTexture,
            state::PipelineState,
        },
        RenderPassStatistics,
    },
    scene::mesh::surface::SurfaceData,
};
use std::{cell::RefCell, rc::Rc};

struct FxaaShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub screen_texture: UniformLocation,
    pub inverse_screen_size: UniformLocation,
}

impl FxaaShader {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/fxaa_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");

        let program = GpuProgram::from_source(state, "FXAAShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            screen_texture: program
                .uniform_location(state, &ImmutableString::new("screenTexture"))?,
            inverse_screen_size: program
                .uniform_location(state, &ImmutableString::new("inverseScreenSize"))?,
            program,
        })
    }
}

pub struct FxaaRenderer {
    shader: FxaaShader,
    quad: GeometryBuffer,
}

impl FxaaRenderer {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            shader: FxaaShader::new(state)?,
            quad: GeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
        })
    }

    pub(crate) fn render(
        &self,
        state: &PipelineState,
        viewport: Rect<i32>,
        frame_texture: Rc<RefCell<GpuTexture>>,
        frame_buffer: &mut FrameBuffer,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

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
            &self.quad,
            state,
            viewport,
            &self.shader.program,
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
                    .set_matrix4(&self.shader.wvp_matrix, &frame_matrix)
                    .set_vector2(
                        &self.shader.inverse_screen_size,
                        &Vector2::new(1.0 / viewport.w() as f32, 1.0 / viewport.h() as f32),
                    )
                    .set_texture(&self.shader.screen_texture, &frame_texture);
            },
        )?;

        Ok(statistics)
    }
}
