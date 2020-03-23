use std::{
    ffi::CString,
    marker::PhantomData,
    rc::Rc,
    cell::RefCell
};
use crate::{
    core::{
        math::{
            vec4::Vec4,
            mat4::Mat4,
            vec3::Vec3,
            vec2::Vec2,
        },
        color::Color
    },
    renderer::{
        error::RendererError,
        framework::{
            gpu_texture::GpuTexture,
            gl::{
                self,
                types::{
                    GLuint,
                    GLint,
                }
            },
            state::State
        }
    },
    utils::log::Log,
};
use rg3d_core::math::mat3::Mat3;

pub struct GpuProgram {
    id: GLuint,
    name_buf: RefCell<Vec<u8>>,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

#[derive(Copy, Clone)]
pub struct UniformLocation {
    id: GLint,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

#[allow(dead_code)]
pub enum UniformValue<'a> {
    Sampler {
        index: usize,
        texture: Rc<RefCell<GpuTexture>>,
    },

    Bool(bool),
    Integer(i32),
    Float(f32),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    Color(Color),
    Mat4(Mat4),
    Mat3(Mat3),

    IntegerArray(&'a [i32]),
    FloatArray(&'a [f32]),
    Vec2Array(&'a [Vec2]),
    Vec3Array(&'a [Vec3]),
    Vec4Array(&'a [Vec4]),
    Mat4Array(&'a [Mat4])
}

impl GpuProgram {
    fn create_shader(name: String, actual_type: GLuint, source: &str) -> Result<GLuint, RendererError> {
        unsafe {
            let csource = CString::new(source)?;

            let shader = gl::CreateShader(actual_type);
            gl::ShaderSource(shader, 1, &csource.as_ptr(), std::ptr::null());
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
                Log::writeln(format!("Failed to compile {} shader: {}", name, compilation_message));
                Err(RendererError::ShaderCompilationFailed {
                    shader_name: name,
                    error_message: compilation_message,
                })
            } else {
                Log::writeln(format!("Shader {} compiled!", name));
                Ok(shader)
            }
        }
    }

    pub fn from_source(name: &str, vertex_source: &str, fragment_source: &str) -> Result<GpuProgram, RendererError> {
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
                    id: program,
                    name_buf: Default::default(),
                    thread_mark: PhantomData,
                })
            }
        }
    }

    pub fn uniform_location(&self, name: &str) -> Result<UniformLocation, RendererError> {
        // Form c string in special buffer to reduce memory allocations
        let buf = &mut self.name_buf.borrow_mut();
        buf.clear();
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        unsafe {
            let id = gl::GetUniformLocation(self.id, buf.as_ptr() as *const i8);
           // if id < 0 {
           //     Err(RendererError::UnableToFindShaderUniform(name.to_owned()))
           // } else {
                Ok(UniformLocation { id, thread_mark: PhantomData })
           // }
        }
    }

    pub fn bind(&mut self, state: &mut State) {
        state.set_program(self.id);
    }

    pub fn set_uniform(&mut self, state: &mut State, location: UniformLocation, value: &UniformValue<'_>) {
        state.set_program(self.id);

        let location = location.id;
        unsafe {
            match value {
                UniformValue::Sampler { index, texture } => {
                    gl::Uniform1i(location, *index as i32);
                    texture.borrow().bind(state,*index);
                }
                UniformValue::Bool(value) => {
                    gl::Uniform1i(location, if *value { gl::TRUE } else { gl::FALSE } as i32);
                }
                UniformValue::Integer(value) => {
                    gl::Uniform1i(location, *value);
                }
                UniformValue::Float(value) => {
                    gl::Uniform1f(location, *value);
                }
                UniformValue::Vec2(value) => {
                    gl::Uniform2f(location, value.x, value.y);
                }
                UniformValue::Vec3(value) => {
                    gl::Uniform3f(location, value.x, value.y, value.z);
                }
                UniformValue::Vec4(value) => {
                    gl::Uniform4f(location, value.x, value.y, value.z, value.w);
                }
                UniformValue::IntegerArray(value) => {
                    gl::Uniform1iv(location, value.len() as i32, value.as_ptr());
                }
                UniformValue::FloatArray(value) => {
                    gl::Uniform1fv(location, value.len() as i32, value.as_ptr());
                }
                UniformValue::Vec2Array(value) => {
                    gl::Uniform2fv(location, value.len() as i32, value.as_ptr() as *const _);
                }
                UniformValue::Vec3Array(value) => {
                    gl::Uniform3fv(location, value.len() as i32, value.as_ptr() as *const _);
                }
                UniformValue::Vec4Array(value) => {
                    gl::Uniform4fv(location, value.len() as i32, value.as_ptr() as *const _);
                }
                UniformValue::Mat4(value) => {
                    gl::UniformMatrix4fv(location, 1, gl::FALSE, &value.f as *const _);
                }
                UniformValue::Mat3(value) => {
                    gl::UniformMatrix3fv(location, 1, gl::FALSE, &value.f as *const _);
                }
                UniformValue::Mat4Array(value) => {
                    gl::UniformMatrix4fv(location, value.len() as i32, gl::FALSE, value.as_ptr() as *const _);
                }
                UniformValue::Color(value) => {
                    let rgba = value.as_frgba();
                    gl::Uniform4f(location, rgba.x, rgba.y, rgba.z, rgba.w);
                }
            }
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