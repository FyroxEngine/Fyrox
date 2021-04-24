use crate::renderer::framework::state::PipelineState;
use crate::renderer::{
    error::RendererError,
    framework::gpu_program::{GpuProgram, UniformLocation},
};

pub struct FlatShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub diffuse_texture: UniformLocation,
}

impl FlatShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, RendererError> {
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
