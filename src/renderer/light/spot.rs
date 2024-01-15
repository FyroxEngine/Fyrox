use crate::core::sstorage::ImmutableString;
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
    pub shadow_alpha: UniformLocation,
}

impl SpotLightShader {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/deferred_spot_light_fs.glsl");
        let vertex_source = include_str!("../shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source(state, "SpotLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            depth_sampler: program
                .uniform_location(state, &ImmutableString::new("depthTexture"))?,
            color_sampler: program
                .uniform_location(state, &ImmutableString::new("colorTexture"))?,
            normal_sampler: program
                .uniform_location(state, &ImmutableString::new("normalTexture"))?,
            material_sampler: program
                .uniform_location(state, &ImmutableString::new("materialTexture"))?,
            spot_shadow_texture: program
                .uniform_location(state, &ImmutableString::new("spotShadowTexture"))?,
            cookie_enabled: program
                .uniform_location(state, &ImmutableString::new("cookieEnabled"))?,
            cookie_texture: program
                .uniform_location(state, &ImmutableString::new("cookieTexture"))?,
            light_view_proj_matrix: program
                .uniform_location(state, &ImmutableString::new("lightViewProjMatrix"))?,
            shadows_enabled: program
                .uniform_location(state, &ImmutableString::new("shadowsEnabled"))?,
            soft_shadows: program.uniform_location(state, &ImmutableString::new("softShadows"))?,
            shadow_map_inv_size: program
                .uniform_location(state, &ImmutableString::new("shadowMapInvSize"))?,
            light_position: program.uniform_location(state, &ImmutableString::new("lightPos"))?,
            light_radius: program.uniform_location(state, &ImmutableString::new("lightRadius"))?,
            light_color: program.uniform_location(state, &ImmutableString::new("lightColor"))?,
            light_direction: program
                .uniform_location(state, &ImmutableString::new("lightDirection"))?,
            half_hotspot_cone_angle_cos: program
                .uniform_location(state, &ImmutableString::new("halfHotspotConeAngleCos"))?,
            half_cone_angle_cos: program
                .uniform_location(state, &ImmutableString::new("halfConeAngleCos"))?,
            inv_view_proj_matrix: program
                .uniform_location(state, &ImmutableString::new("invViewProj"))?,
            camera_position: program
                .uniform_location(state, &ImmutableString::new("cameraPosition"))?,
            shadow_bias: program.uniform_location(state, &ImmutableString::new("shadowBias"))?,
            light_intensity: program
                .uniform_location(state, &ImmutableString::new("lightIntensity"))?,
            shadow_alpha: program.uniform_location(state, &ImmutableString::new("shadowAlpha"))?,
            program,
        })
    }
}
