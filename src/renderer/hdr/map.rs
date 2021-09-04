use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct MapShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub hdr_sampler: UniformLocation,
    pub lum_sampler: UniformLocation,
    pub bloom_sampler: UniformLocation,
    pub color_map_sampler: UniformLocation,
    pub use_color_grading: UniformLocation,
    pub key_value: UniformLocation,
    pub min_luminance: UniformLocation,
    pub max_luminance: UniformLocation,
    pub auto_exposure: UniformLocation,
    pub fixed_exposure: UniformLocation,
}

impl MapShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_map.glsl");
        let vertex_source = include_str!("../shaders/flat_vs.glsl");

        let program =
            GpuProgram::from_source(state, "HdrToLdrShader", vertex_source, fragment_source)?;

        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            hdr_sampler: program.uniform_location(state, "hdrSampler")?,
            lum_sampler: program.uniform_location(state, "lumSampler")?,
            bloom_sampler: program.uniform_location(state, "bloomSampler")?,
            color_map_sampler: program.uniform_location(state, "colorMapSampler")?,
            use_color_grading: program.uniform_location(state, "useColorGrading")?,
            key_value: program.uniform_location(state, "keyValue")?,
            min_luminance: program.uniform_location(state, "minLuminance")?,
            max_luminance: program.uniform_location(state, "maxLuminance")?,
            auto_exposure: program.uniform_location(state, "autoExposure")?,
            fixed_exposure: program.uniform_location(state, "fixedExposure")?,
            program,
        })
    }
}
