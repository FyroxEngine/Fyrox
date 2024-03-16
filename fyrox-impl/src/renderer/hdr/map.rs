use crate::core::sstorage::ImmutableString;
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
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_map.glsl");
        let vertex_source = include_str!("../shaders/flat_vs.glsl");

        let program =
            GpuProgram::from_source(state, "HdrToLdrShader", vertex_source, fragment_source)?;

        Ok(Self {
            wvp_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            hdr_sampler: program.uniform_location(state, &ImmutableString::new("hdrSampler"))?,
            lum_sampler: program.uniform_location(state, &ImmutableString::new("lumSampler"))?,
            bloom_sampler: program
                .uniform_location(state, &ImmutableString::new("bloomSampler"))?,
            color_map_sampler: program
                .uniform_location(state, &ImmutableString::new("colorMapSampler"))?,
            use_color_grading: program
                .uniform_location(state, &ImmutableString::new("useColorGrading"))?,
            key_value: program.uniform_location(state, &ImmutableString::new("keyValue"))?,
            min_luminance: program
                .uniform_location(state, &ImmutableString::new("minLuminance"))?,
            max_luminance: program
                .uniform_location(state, &ImmutableString::new("maxLuminance"))?,
            auto_exposure: program
                .uniform_location(state, &ImmutableString::new("autoExposure"))?,
            fixed_exposure: program
                .uniform_location(state, &ImmutableString::new("fixedExposure"))?,
            program,
        })
    }
}
