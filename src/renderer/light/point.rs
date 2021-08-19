use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct PointLightShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub depth_sampler: UniformLocation,
    pub color_sampler: UniformLocation,
    pub normal_sampler: UniformLocation,
    pub point_shadow_texture: UniformLocation,
    pub shadows_enabled: UniformLocation,
    pub soft_shadows: UniformLocation,
    pub light_position: UniformLocation,
    pub light_radius: UniformLocation,
    pub light_color: UniformLocation,
    pub inv_view_proj_matrix: UniformLocation,
    pub camera_position: UniformLocation,
    pub shadow_bias: UniformLocation,
    pub light_intensity: UniformLocation,
}

impl PointLightShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/deferred_point_light_fs.glsl");
        let vertex_source = include_str!("../shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source(state, "PointLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            depth_sampler: program.uniform_location(state, "depthTexture")?,
            color_sampler: program.uniform_location(state, "colorTexture")?,
            normal_sampler: program.uniform_location(state, "normalTexture")?,
            point_shadow_texture: program.uniform_location(state, "pointShadowTexture")?,
            shadows_enabled: program.uniform_location(state, "shadowsEnabled")?,
            soft_shadows: program.uniform_location(state, "softShadows")?,
            light_position: program.uniform_location(state, "lightPos")?,
            light_radius: program.uniform_location(state, "lightRadius")?,
            light_color: program.uniform_location(state, "lightColor")?,
            inv_view_proj_matrix: program.uniform_location(state, "invViewProj")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
            shadow_bias: program.uniform_location(state, "shadowBias")?,
            light_intensity: program.uniform_location(state, "lightIntensity")?,
            program,
        })
    }
}
