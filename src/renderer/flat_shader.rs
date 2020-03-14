use crate::{
    renderer::{
        gpu_program::{GpuProgram, UniformLocation},
        error::RendererError
    },
    core::math::mat4::Mat4
};

pub struct FlatShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
}

impl FlatShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/flat_fs.glsl");
        let vertex_source =include_str!("shaders/flat_vs.glsl");

        let mut program = GpuProgram::from_source("FlatShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.get_uniform_location("worldViewProjection")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            program,
        })
    }

    pub fn bind(&mut self) -> &mut Self {
        self.program.bind();
        self
    }

    pub fn set_wvp_matrix(&mut self, mat: &Mat4) -> &mut Self {
        self.program.set_mat4(self.wvp_matrix, mat);
        self
    }

    pub fn set_diffuse_texture(&mut self, id: i32) -> &mut Self  {
        self.program.set_int(self.diffuse_texture, id);
        self
    }
}