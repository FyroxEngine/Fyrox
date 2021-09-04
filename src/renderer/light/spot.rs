use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct SpotLightShader {
    pub program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub depth_sampler: UniformLocation,
    pub color_sampler: UniformLocation,
    pub normal_sampler: UniformLocation,
    pub material_sampler: UniformLocation,
    pub spot_shadow_texture: UniformLocation,
    pub cookie_enabled: UniformLocation,
    pub cookie_texture: UniformLocation,
    pub light_view_proj_matrix: UniformLocation,
    pub shadows_enabled: UniformLocation,
    pub soft_shadows: UniformLocation,
    pub shadow_map_inv_size: UniformLocation,
    pub light_position: UniformLocation,
    pub light_radius: UniformLocation,
    pub light_color: UniformLocation,
    pub light_direction: UniformLocation,
    pub half_hotspot_cone_angle_cos: UniformLocation,
    pub half_cone_angle_cos: UniformLocation,
    pub inv_view_proj_matrix: UniformLocation,
    pub camera_position: UniformLocation,
    pub shadow_bias: UniformLocation,
    pub light_intensity: UniformLocation,
}

impl SpotLightShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/deferred_spot_light_fs.glsl");
        let vertex_source = include_str!("../shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source(state, "SpotLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            depth_sampler: program.uniform_location(state, "depthTexture")?,
            color_sampler: program.uniform_location(state, "colorTexture")?,
            normal_sampler: program.uniform_location(state, "normalTexture")?,
            material_sampler: program.uniform_location(state, "materialTexture")?,
            spot_shadow_texture: program.uniform_location(state, "spotShadowTexture")?,
            cookie_enabled: program.uniform_location(state, "cookieEnabled")?,
            cookie_texture: program.uniform_location(state, "cookieTexture")?,
            light_view_proj_matrix: program.uniform_location(state, "lightViewProjMatrix")?,
            shadows_enabled: program.uniform_location(state, "shadowsEnabled")?,
            soft_shadows: program.uniform_location(state, "softShadows")?,
            shadow_map_inv_size: program.uniform_location(state, "shadowMapInvSize")?,
            light_position: program.uniform_location(state, "lightPos")?,
            light_radius: program.uniform_location(state, "lightRadius")?,
            light_color: program.uniform_location(state, "lightColor")?,
            light_direction: program.uniform_location(state, "lightDirection")?,
            half_hotspot_cone_angle_cos: program
                .uniform_location(state, "halfHotspotConeAngleCos")?,
            half_cone_angle_cos: program.uniform_location(state, "halfConeAngleCos")?,
            inv_view_proj_matrix: program.uniform_location(state, "invViewProj")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
            shadow_bias: program.uniform_location(state, "shadowBias")?,
            light_intensity: program.uniform_location(state, "lightIntensity")?,
            program,
        })
    }
}
