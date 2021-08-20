use crate::renderer::framework::error::FrameworkError;
use crate::renderer::framework::gpu_program::{GpuProgram, UniformLocation};
use crate::renderer::framework::state::PipelineState;

pub struct MapShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub hdr_sampler: UniformLocation,
    pub lum_sampler: UniformLocation,
}

impl MapShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_map.glsl");
        let vertex_source = include_str!("../shaders/flat_vs.glsl");

        let program =
            GpuProgram::from_source(state, "AdaptationShader", vertex_source, fragment_source)?;

        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            hdr_sampler: program.uniform_location(state, "hdrSampler")?,
            lum_sampler: program.uniform_location(state, "lumSampler")?,
            program,
        })
    }
}
