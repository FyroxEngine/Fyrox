use crate::core::algebra::{Matrix3, Matrix4, Vector2, Vector3, Vector4};
use crate::utils::log::MessageKind;
use crate::{
    core::color::Color,
    renderer::{
        error::RendererError,
        framework::{
            gl::{
                self,
                types::{GLint, GLuint},
            },
            gpu_texture::GpuTexture,
            state::PipelineState,
        },
    },
    utils::log::Log,
};
use std::{cell::RefCell, ffi::CString, marker::PhantomData, rc::Rc};

pub struct GpuProgram {
    id: GLuint,
    name_buf: RefCell<Vec<u8>>,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

#[derive(Copy, Clone, Debug)]
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
    Vector2(Vector2<f32>),
    Vector3(Vector3<f32>),
    Vector4(Vector4<f32>),
    Color(Color),
    Matrix4(Matrix4<f32>),
    Matrix3(Matrix3<f32>),

    IntegerArray(&'a [i32]),
    FloatArray(&'a [f32]),
    Vec2Array(&'a [Vector2<f32>]),
    Vec3Array(&'a [Vector3<f32>]),
    Vec4Array(&'a [Vector4<f32>]),
    Mat4Array(&'a [Matrix4<f32>]),
}

fn create_shader(name: String, actual_type: GLuint, source: &str) -> Result<GLuint, RendererError> {
    unsafe {
        let csource = prepare_source_code(source)?;

        let shader = gl::CreateShader(actual_type);
        gl::ShaderSource(shader, 1, &csource.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        let mut status = 1;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        let mut log_len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut log_len);
        let mut buffer: Vec<u8> = Vec::with_capacity(log_len as usize);
        buffer.set_len(log_len as usize);
        gl::GetShaderInfoLog(
            shader,
            log_len,
            std::ptr::null_mut(),
            buffer.as_mut_ptr() as *mut i8,
        );
        let compilation_message = String::from_utf8_unchecked(buffer);

        if status == 0 {
            Log::writeln(
                MessageKind::Error,
                format!("Failed to compile {} shader: {}", name, compilation_message),
            );
            Err(RendererError::ShaderCompilationFailed {
                shader_name: name,
                error_message: compilation_message,
            })
        } else {
            Log::writeln(
                MessageKind::Information,
                format!("Shader {} compiled!\n{}", name, compilation_message),
            );
            Ok(shader)
        }
    }
}

fn prepare_source_code(code: &str) -> Result<CString, RendererError> {
    let mut shared = "\n// include 'shared.glsl'\n".to_owned();
    shared += include_str!("../shaders/shared.glsl");
    shared += "\n// end of include\n";

    if let Some(p) = code.rfind('#') {
        let mut full = code.to_owned();
        let end = p + full[p..].find('\n').unwrap() + 1;
        full.insert_str(end, &shared);
        Ok(CString::new(full)?)
    } else {
        shared += code;
        Ok(CString::new(shared)?)
    }
}

impl GpuProgram {
    pub fn from_source(
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<GpuProgram, RendererError> {
        unsafe {
            let vertex_shader = create_shader(
                format!("{}_VertexShader", name),
                gl::VERTEX_SHADER,
                vertex_source,
            )?;
            let fragment_shader = create_shader(
                format!("{}_FragmentShader", name),
                gl::FRAGMENT_SHADER,
                fragment_source,
            )?;
            let program: GLuint = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::DeleteShader(vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::DeleteShader(fragment_shader);
            gl::LinkProgram(program);
            let mut status = 1;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

            let mut log_len = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut log_len);
            let mut buffer: Vec<u8> = Vec::with_capacity(log_len as usize);
            gl::GetProgramInfoLog(
                program,
                log_len,
                std::ptr::null_mut(),
                buffer.as_mut_ptr() as *mut i8,
            );
            let link_message = String::from_utf8_lossy(&buffer).to_string();
            if status == 0 {
                Log::writeln(
                    MessageKind::Error,
                    format!("Failed to link {} shader: {}", name, link_message),
                );
                Err(RendererError::ShaderLinkingFailed {
                    shader_name: name.to_owned(),
                    error_message: link_message,
                })
            } else {
                Log::writeln(
                    MessageKind::Information,
                    format!("Shader {} linked!\n{}", name, link_message),
                );
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
            if id < 0 {
                Err(RendererError::UnableToFindShaderUniform(name.to_owned()))
            } else {
                Ok(UniformLocation {
                    id,
                    thread_mark: PhantomData,
                })
            }
        }
    }

    pub fn bind(&self, state: &mut PipelineState) {
        state.set_program(self.id);
    }

    pub fn set_uniform(
        &self,
        state: &mut PipelineState,
        location: UniformLocation,
        value: &UniformValue<'_>,
    ) {
        state.set_program(self.id);

        let location = location.id;
        unsafe {
            match value {
                UniformValue::Sampler { index, texture } => {
                    gl::Uniform1i(location, *index as i32);
                    texture.borrow().bind(state, *index);
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
                UniformValue::Vector2(value) => {
                    gl::Uniform2f(location, value.x, value.y);
                }
                UniformValue::Vector3(value) => {
                    gl::Uniform3f(location, value.x, value.y, value.z);
                }
                UniformValue::Vector4(value) => {
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
                UniformValue::Matrix4(value) => {
                    gl::UniformMatrix4fv(location, 1, gl::FALSE, value.as_ptr() as *const _);
                }
                UniformValue::Matrix3(value) => {
                    gl::UniformMatrix3fv(location, 1, gl::FALSE, value.as_ptr() as *const _);
                }
                UniformValue::Mat4Array(value) => {
                    gl::UniformMatrix4fv(
                        location,
                        value.len() as i32,
                        gl::FALSE,
                        value.as_ptr() as *const _,
                    );
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
