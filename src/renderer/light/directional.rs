use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct DirectionalLightShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub depth_sampler: UniformLocation,
    pub color_sampler: UniformLocation,
    pub normal_sampler: UniformLocation,
    pub light_direction: UniformLocation,
    pub light_color: UniformLocation,
    pub inv_view_proj_matrix: UniformLocation,
    pub camera_position: UniformLocation,
    pub light_intensity: UniformLocation,
}

impl DirectionalLightShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/deferred_directional_light_fs.glsl");
        let vertex_source = include_str!("../shaders/deferred_light_vs.glsl");
        let program = GpuProgram::from_source(
            state,
            "DirectionalLightShader",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            depth_sampler: program.uniform_location(state, "depthTexture")?,
            color_sampler: program.uniform_location(state, "colorTexture")?,
            normal_sampler: program.uniform_location(state, "normalTexture")?,
            light_direction: program.uniform_location(state, "lightDirection")?,
            light_color: program.uniform_location(state, "lightColor")?,
            inv_view_proj_matrix: program.uniform_location(state, "invViewProj")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
            light_intensity: program.uniform_location(state, "lightIntensity")?,
            program,
        })
    }
}
