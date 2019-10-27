use std::ffi::CStr;
use crate::{
    renderer::{
        gl::types::GLuint,
        gl,
        gl::types::GLint,
        gl::types::GLfloat,
    },
};
use crate::renderer::error::RendererError;

use rg3d_core::{
    math::{
        vec4::Vec4,
        mat4::Mat4,
        vec3::Vec3,
        vec2::Vec2,
    },
};

pub struct GpuProgram {
    name: String,
    id: GLuint,
    name_buf: Vec<u8>,
}

#[derive(Copy, Clone)]
pub struct UniformLocation {
    id: GLint
}

impl GpuProgram {
    fn create_shader(name: String, actual_type: GLuint, source: &CStr) -> Result<GLuint, RendererError> {
        unsafe {
            let shader = gl::CreateShader(actual_type);
            gl::ShaderSource(shader, 1, &source.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);

            let mut status = 1;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
            if status == 0 {
                let mut log_len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut log_len);
                let mut buffer: Vec<u8> = Vec::with_capacity(log_len as usize);
                buffer.set_len(log_len as usize);
                gl::GetShaderInfoLog(shader, log_len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
                let compilation_message = String::from_utf8_unchecked(buffer);
                println!("Failed to compile {} shader: {}", name, compilation_message);
                Err(RendererError::ShaderCompilationFailed {
                    shader_name: name,
                    error_message: compilation_message,
                })
            } else {
                println!("Shader {} compiled!", name);
                Ok(shader)
            }
        }
    }

    pub fn from_source(name: &str, vertex_source: &CStr, fragment_source: &CStr) -> Result<GpuProgram, RendererError> {
        unsafe {
            let vertex_shader = Self::create_shader(format!("{}_VertexShader", name), gl::VERTEX_SHADER, vertex_source)?;
            let fragment_shader = Self::create_shader(format!("{}_FragmentShader", name), gl::FRAGMENT_SHADER, fragment_source)?;
            let program: GLuint = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::DeleteShader(vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::DeleteShader(fragment_shader);
            gl::LinkProgram(program);
            let mut status = 1;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
            if status == 0 {
                let mut log_len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut log_len);
                let mut buffer: Vec<u8> = Vec::with_capacity(log_len as usize);
                gl::GetProgramInfoLog(program, log_len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
                Err(RendererError::ShaderLinkingFailed {
                    shader_name: name.to_owned(),
                    error_message: String::from_utf8_unchecked(buffer),
                })
            } else {
                Ok(Self {
                    name: name.to_owned(),
                    id: program,
                    name_buf: Vec::new(),
                })
            }
        }
    }

    pub fn get_uniform_location(&mut self, name: &str) -> Result<UniformLocation, RendererError> {
        // Form c string in special buffer to reduce memory allocations
        let buf = &mut self.name_buf;
        buf.clear();
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        unsafe {
            let id = gl::GetUniformLocation(self.id, buf.as_ptr() as *const i8);
            if id < 0 {
                Err(RendererError::UnableToFindShaderUniform(name.to_owned()))
            } else {
                Ok(UniformLocation { id })
            }
        }
    }

    pub fn bind(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn set_mat4(&self, location: UniformLocation, mat: &Mat4) {
        unsafe {
            gl::UniformMatrix4fv(location.id, 1, gl::FALSE, &mat.f as *const GLfloat);
        }
    }

    pub fn set_mat4_array(&self, location: UniformLocation, mat: &[Mat4]) {
        unsafe {
            gl::UniformMatrix4fv(location.id, mat.len() as i32, gl::FALSE, mat[0].f.as_ptr() as *const GLfloat);
        }
    }

    pub fn set_int(&self, location: UniformLocation, value: i32) {
        unsafe {
            gl::Uniform1i(location.id, value);
        }
    }

    pub fn set_vec4(&self, location: UniformLocation, value: &Vec4) {
        unsafe {
            gl::Uniform4f(location.id, value.x, value.y, value.z, value.w);
        }
    }

    pub fn set_bool(&self, location: UniformLocation, value: bool) {
        self.set_int(location,  i32::from(if value { gl::TRUE } else { gl::FALSE }))
    }

    pub fn set_float(&self, location: UniformLocation, value: f32) {
        unsafe {
            gl::Uniform1f(location.id, value)
        }
    }

    pub fn set_vec3(&self, location: UniformLocation, value: &Vec3) {
        unsafe {
            gl::Uniform3f(location.id, value.x, value.y, value.z)
        }
    }

    pub fn set_vec2(&self, location: UniformLocation, value: Vec2) {
        unsafe {
            gl::Uniform2f(location.id, value.x, value.y)
        }
    }
}

impl Drop for GpuProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}