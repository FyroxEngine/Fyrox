use crate::core::sstorage::ImmutableString;
use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct SkyboxShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub cubemap_texture: UniformLocation,
}

impl SkyboxShader {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/skybox_fs.glsl");
        let vertex_source = include_str!("shaders/skybox_vs.glsl");

        let program =
            GpuProgram::from_source(state, "SkyboxShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            cubemap_texture: program
                .uniform_location(state, &ImmutableString::new("cubemapTexture"))?,
            program,
        })
    }
}
