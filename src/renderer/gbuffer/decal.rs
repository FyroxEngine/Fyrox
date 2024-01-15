use crate::core::sstorage::ImmutableString;
use crate::renderer::framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
    state::PipelineState,
};

pub struct DecalShader {
    pub world_view_projection: UniformLocation,
    pub scene_depth: UniformLocation,
    pub diffuse_texture: UniformLocation,
    pub normal_texture: UniformLocation,
    pub inv_view_proj: UniformLocation,
    pub inv_world_decal: UniformLocation,
    pub resolution: UniformLocation,
    pub color: UniformLocation,
    pub layer_index: UniformLocation,
    pub decal_mask: UniformLocation,
    pub program: GpuProgram,
}

impl DecalShader {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/decal_fs.glsl");
        let vertex_source = include_str!("../shaders/decal_vs.glsl");

        let program =
            GpuProgram::from_source(state, "DecalShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_view_projection: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            scene_depth: program.uniform_location(state, &ImmutableString::new("sceneDepth"))?,
            diffuse_texture: program
                .uniform_location(state, &ImmutableString::new("diffuseTexture"))?,
            normal_texture: program
                .uniform_location(state, &ImmutableString::new("normalTexture"))?,
            inv_view_proj: program.uniform_location(state, &ImmutableString::new("invViewProj"))?,
            inv_world_decal: program
                .uniform_location(state, &ImmutableString::new("invWorldDecal"))?,
            resolution: program.uniform_location(state, &ImmutableString::new("resolution"))?,
            color: program.uniform_location(state, &ImmutableString::new("color"))?,
            layer_index: program.uniform_location(state, &ImmutableString::new("layerIndex"))?,
            decal_mask: program.uniform_location(state, &ImmutableString::new("decalMask"))?,
            program,
        })
    }
}
