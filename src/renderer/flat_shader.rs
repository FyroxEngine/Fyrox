use crate::rendering_framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct FlatShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub diffuse_texture: UniformLocation,
}

impl FlatShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/flat_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");

        let program = GpuProgram::from_source(state, "FlatShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            program,
        })
    }
}
