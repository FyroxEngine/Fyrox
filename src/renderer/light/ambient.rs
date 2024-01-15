use crate::core::sstorage::ImmutableString;
use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct AmbientLightShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub diffuse_texture: UniformLocation,
    pub ambient_color: UniformLocation,
    pub ao_sampler: UniformLocation,
    pub ambient_texture: UniformLocation,
}

impl AmbientLightShader {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/ambient_light_fs.glsl");
        let vertex_source = include_str!("../shaders/ambient_light_vs.glsl");
        let program =
            GpuProgram::from_source(state, "AmbientLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            diffuse_texture: program
                .uniform_location(state, &ImmutableString::new("diffuseTexture"))?,
            ambient_color: program
                .uniform_location(state, &ImmutableString::new("ambientColor"))?,
            ao_sampler: program.uniform_location(state, &ImmutableString::new("aoSampler"))?,
            ambient_texture: program
                .uniform_location(state, &ImmutableString::new("ambientTexture"))?,
            program,
        })
    }
}
