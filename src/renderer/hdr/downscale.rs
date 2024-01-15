use crate::core::sstorage::ImmutableString;
use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct DownscaleShader {
    pub program: GpuProgram,
    pub lum_sampler: UniformLocation,
    pub inv_size: UniformLocation,
    pub wvp_matrix: UniformLocation,
}

impl DownscaleShader {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_downscale_fs.glsl");
        let vertex_source = include_str!("../shaders/flat_vs.glsl");

        let program =
            GpuProgram::from_source(state, "DownscaleShader", vertex_source, fragment_source)?;

        Ok(Self {
            wvp_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            lum_sampler: program.uniform_location(state, &ImmutableString::new("lumSampler"))?,
            inv_size: program.uniform_location(state, &ImmutableString::new("invSize"))?,
            program,
        })
    }
}
