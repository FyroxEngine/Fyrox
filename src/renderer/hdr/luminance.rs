use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct LuminanceShader {
    program: GpuProgram,
    frame_sampler: UniformLocation,
    inv_size: UniformLocation,
}

impl LuminanceShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_luminance_fs.glsl");
        let vertex_source = include_str!("../shaders/flat_vs.glsl");

        let program =
            GpuProgram::from_source(state, "LuminanceShader", vertex_source, fragment_source)?;

        Ok(Self {
            frame_sampler: program.uniform_location(state, "frameSampler")?,
            inv_size: program.uniform_location(state, "invSize")?,
            program,
        })
    }
}
