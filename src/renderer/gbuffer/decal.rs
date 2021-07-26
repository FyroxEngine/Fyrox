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
    pub normal_matrix_decal: UniformLocation,
    pub resolution: UniformLocation,
    pub program: GpuProgram,
}

impl DecalShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/decal_fs.glsl");
        let vertex_source = include_str!("../shaders/decal_vs.glsl");

        let program =
            GpuProgram::from_source(state, "DecalShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_view_projection: program.uniform_location(state, "worldViewProjection")?,
            scene_depth: program.uniform_location(state, "sceneDepth")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            normal_texture: program.uniform_location(state, "normalTexture")?,
            inv_view_proj: program.uniform_location(state, "invViewProj")?,
            inv_world_decal: program.uniform_location(state, "invWorldDecal")?,
            resolution: program.uniform_location(state, "resolution")?,
            normal_matrix_decal: program.uniform_location(state, "normalMatrixDecal")?,
            program,
        })
    }
}
