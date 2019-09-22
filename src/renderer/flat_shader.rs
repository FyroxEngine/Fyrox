use crate::{
    renderer::{
        gpu_program::{GpuProgram, UniformLocation},
        error::RendererError
    }
};
use std::ffi::CString;
use rg3d_core::math::mat4::Mat4;

pub struct FlatShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
}

impl FlatShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = CString::new(r#"
        #version 330 core

        uniform sampler2D diffuseTexture;

        out vec4 FragColor;

        in vec2 texCoord;

        void main()
        {
            FragColor = texture(diffuseTexture, texCoord);
        }
        "#)?;

        let vertex_source = CString::new(r#"
        #version 330 core

        layout(location = 0) in vec3 vertexPosition;
        layout(location = 1) in vec2 vertexTexCoord;

        uniform mat4 worldViewProjection;

        out vec2 texCoord;

        void main()
        {
            texCoord = vertexTexCoord;
            gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
        }
        "#)?;

        let mut program = GpuProgram::from_source("FlatShader", &vertex_source, &fragment_source)?;
        Ok(Self {
            wvp_matrix: program.get_uniform_location("worldViewProjection")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            program,
        })
    }

    pub fn bind(&self) {
        self.program.bind();
    }

    pub fn set_wvp_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.wvp_matrix, mat)
    }

    pub fn set_diffuse_texture(&self, id: i32) {
        self.program.set_int(self.diffuse_texture, id)
    }
}