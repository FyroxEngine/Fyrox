use crate::{
    renderer::{
        gl::types::GLuint,
        gl,
        gl::types::GLint,
        gl::types::GLfloat,
    },
    math::mat4::Mat4,
};
use std::ffi::CStr;
use crate::math::vec4::Vec4;
use crate::math::vec3::Vec3;

pub struct GpuProgram {
    id: GLuint,
    name_buf: Vec<u8>,
}

#[derive(Copy, Clone)]
pub struct UniformLocation {
    id: GLint
}

impl GpuProgram {
    pub fn create_shader(actual_type: GLuint, source: &CStr) -> Result<GLuint, String> {
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
                println!("Failed to compile shader: {}", compilation_message);
                Err(compilation_message)
            } else {
                println!("Shader compiled!");
                Ok(shader)
            }
        }
    }

    pub fn from_source(vertex_source: &CStr, fragment_source: &CStr) -> Result<GpuProgram, String> {
        unsafe {
            let vertex_shader = Self::create_shader(gl::VERTEX_SHADER, vertex_source)?;
            let fragment_shader = Self::create_shader(gl::FRAGMENT_SHADER, fragment_source)?;
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
                Err(String::from_utf8_unchecked(buffer))
            } else {
                Ok(Self {
                    id: program,
                    name_buf: Vec::new(),
                })
            }
        }
    }

    pub fn get_uniform_location(&mut self, name: &str) -> UniformLocation {
        // Form c string in special buffer to reduce memory allocations
        let buf = &mut self.name_buf;
        buf.clear();
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        unsafe {
            UniformLocation { id: gl::GetUniformLocation(self.id, buf.as_ptr() as *const i8) }
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
            gl::UniformMatrix4fv(location.id, mat.len() as i32, gl::FALSE, &mat[0].f as *const GLfloat);
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
}

impl Drop for GpuProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}