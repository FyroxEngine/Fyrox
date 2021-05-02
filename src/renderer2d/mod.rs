use crate::rendering_framework::state::PipelineState;
use crate::rendering_framework::{
    error::FrameworkError,
    gpu_program::{GpuProgram, UniformLocation},
};
use crate::scene2d::Scene2dContainer;

pub struct SpriteShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
}

impl SpriteShader {
    pub fn new(state: &mut PipelineState) -> Result<SpriteShader, FrameworkError> {
        let fragment_source = include_str!("shaders/sprite_fs.glsl");
        let vertex_source = include_str!("shaders/sprite_vs.glsl");

        let program =
            GpuProgram::from_source(state, "SpriteShader2D", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            program,
        })
    }
}

struct Renderer2D {
    shader: SpriteShader,
}

impl Renderer2D {
    pub(in crate) fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            shader: SpriteShader::new(state)?,
        })
    }

    pub fn render(&mut self, scenes: &Scene2dContainer) {
        for _scene in scenes.iter() {}
    }
}
