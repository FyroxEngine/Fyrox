use crate::renderer::{
    gpu_program::{
        GpuProgram,
        UniformLocation,
    },
    error::RendererError,
};

pub struct FlatShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub diffuse_texture: UniformLocation,
}

impl FlatShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/flat_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");

        let mut program = GpuProgram::from_source("FlatShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.get_uniform_location("worldViewProjection")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            program,
        })
    }
}