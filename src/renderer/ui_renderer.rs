use std::ffi::{CString, c_void};
use crate::{
    renderer::{
        gpu_program::{GpuProgram, UniformLocation},
        gl,
        gl::types::GLuint,
        error::RendererError
    },
    gui::draw::{DrawingContext, CommandKind},
};

use rg3d_core::{
    math::{
        mat4::Mat4,
        vec2::Vec2,
    }
};


struct UIShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
}

impl UIShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = CString::new(r#"
        #version 330 core

        uniform sampler2D diffuseTexture;

        out vec4 FragColor;
        in vec2 texCoord;
        in vec4 color;

        void main()
        {
            FragColor = color;
            FragColor.a *= texture(diffuseTexture, texCoord).r;
        };"#)?;


        let vertex_source = CString::new(r#"
        #version 330 core

        layout(location = 0) in vec3 vertexPosition;
        layout(location = 1) in vec2 vertexTexCoord;
        layout(location = 2) in vec4 vertexColor;

        uniform mat4 worldViewProjection;

        out vec2 texCoord;
        out vec4 color;

        void main()
        {
            texCoord = vertexTexCoord;
            color = vertexColor;
            gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
        };"#)?;

        let mut program = GpuProgram::from_source("UIShader", &vertex_source, &fragment_source)?;

        Ok(Self {
            wvp_matrix: program.get_uniform_location("worldViewProjection")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            program,
        })
    }

    pub fn bind(&self) {
        self.program.bind()
    }

    pub fn set_wvp_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.wvp_matrix, mat)
    }

    pub fn set_diffuse_texture_sampler_id(&self, id: i32) {
        self.program.set_int(self.diffuse_texture, id)
    }
}

pub struct UIRenderer {
    shader: UIShader,
    vbo: GLuint,
    vao: GLuint,
    ebo: GLuint,
}

impl UIRenderer {
    pub(in crate::renderer) fn new() -> Result<Self, RendererError> {
        unsafe {
            let mut vbo = 0;
            let mut ebo = 0;
            let mut vao = 0;

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);

            let mut offset = 0;
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE,
                                    DrawingContext::get_vertex_size(),
                                    offset as *const c_void);
            gl::EnableVertexAttribArray(0);
            offset += std::mem::size_of::<Vec2>();

            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, DrawingContext::get_vertex_size(), offset as *const c_void);
            gl::EnableVertexAttribArray(1);
            offset += std::mem::size_of::<Vec2>();

            gl::VertexAttribPointer(2, 4, gl::UNSIGNED_BYTE, gl::TRUE,
                                    DrawingContext::get_vertex_size(),
                                    offset as *const c_void);
            gl::EnableVertexAttribArray(2);

            gl::BindVertexArray(0);

            Ok(Self {
                vao,
                vbo,
                ebo,
                shader: UIShader::new()?,
            })
        }
    }

    pub(in crate::renderer) fn render(&mut self, frame_width: f32, frame_height: f32, drawing_context: &DrawingContext, white_dummy: GLuint) {
        unsafe {
            // Render UI on top of everything
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Disable(gl::CULL_FACE);

            self.shader.bind();
            gl::ActiveTexture(gl::TEXTURE0);

            let index_bytes = drawing_context.get_indices_bytes();
            let vertex_bytes = drawing_context.get_vertices_bytes();

            // Upload to GPU.
            gl::BindVertexArray(self.vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(gl::ARRAY_BUFFER, vertex_bytes, drawing_context.get_vertices_ptr(), gl::DYNAMIC_DRAW);

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, index_bytes, drawing_context.get_indices_ptr(), gl::DYNAMIC_DRAW);

            let ortho = Mat4::ortho(0.0, frame_width, frame_height, 0.0,
                                    -1.0, 1.0);
            self.shader.set_wvp_matrix(&ortho);

            for cmd in drawing_context.get_commands() {
                let index_count = cmd.get_triangle_count() * 3;
                if cmd.get_nesting() != 0 {
                    gl::Enable(gl::STENCIL_TEST);
                } else {
                    gl::Disable(gl::STENCIL_TEST);
                }
                match cmd.get_kind() {
                    CommandKind::Clip => {
                        if cmd.get_nesting() == 1 {
                            gl::Clear(gl::STENCIL_BUFFER_BIT);
                        }
                        gl::StencilOp(gl::KEEP, gl::KEEP, gl::INCR);
                        // Make sure that clipping rect will be drawn at previous nesting level only (clip to parent)
                        gl::StencilFunc(gl::EQUAL, i32::from(cmd.get_nesting() - 1), 0xFF);
                        gl::BindTexture(gl::TEXTURE_2D, white_dummy);
                        // Draw clipping geometry to stencil buffer
                        gl::StencilMask(0xFF);
                        gl::ColorMask(gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
                    }
                    CommandKind::Geometry => {
                        // Make sure to draw geometry only on clipping geometry with current nesting level
                        gl::StencilFunc(gl::EQUAL, i32::from(cmd.get_nesting()), 0xFF);

                        if cmd.get_texture() != 0 {
                            gl::ActiveTexture(gl::TEXTURE0);
                            self.shader.set_diffuse_texture_sampler_id(0);
                            gl::BindTexture(gl::TEXTURE_2D, cmd.get_texture());
                        } else {
                            gl::BindTexture(gl::TEXTURE_2D, white_dummy);
                        }

                        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
                        // Do not draw geometry to stencil buffer
                        gl::StencilMask(0x00);
                    }
                }

                let index_offset_bytes = cmd.get_index_offset() * std::mem::size_of::<GLuint>();
                gl::DrawElements(gl::TRIANGLES, index_count as i32, gl::UNSIGNED_INT,
                                 index_offset_bytes as *const c_void);
            }
            gl::BindVertexArray(0);
        }
    }
}