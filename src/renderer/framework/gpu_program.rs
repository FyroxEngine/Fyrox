use crate::{
    core::{
        algebra::{Matrix3, Matrix4, Vector2, Vector3, Vector4},
        color::Color,
        scope_profile,
    },
    renderer::{
        error::RendererError,
        framework::{gpu_texture::GpuTexture, state::PipelineState},
    },
    utils::log::{Log, MessageKind},
};
use glow::HasContext;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

pub struct GpuProgram {
    state: *mut PipelineState,
    id: glow::Program,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

#[derive(Clone, Debug)]
pub struct UniformLocation {
    id: glow::UniformLocation,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

#[allow(dead_code)]
pub enum UniformValue<'a> {
    Sampler {
        index: u32,
        texture: Rc<RefCell<GpuTexture>>,
    },

    Bool(bool),
    Integer(i32),
    Float(f32),
    Vector2(&'a Vector2<f32>),
    Vector3(&'a Vector3<f32>),
    Vector4(&'a Vector4<f32>),
    Color(Color),
    Matrix4(&'a Matrix4<f32>),
    Matrix3(&'a Matrix3<f32>),

    IntegerArray(&'a [i32]),
    FloatArray(&'a [f32]),
    Vec2Array(&'a [Vector2<f32>]),
    Vec3Array(&'a [Vector3<f32>]),
    Vec4Array(&'a [Vector4<f32>]),
    Mat4Array(&'a [Matrix4<f32>]),
}

unsafe fn create_shader(
    state: &mut PipelineState,
    name: String,
    actual_type: u32,
    source: &str,
) -> Result<glow::Shader, RendererError> {
    let merged_source = prepare_source_code(source);

    let shader = state.gl.create_shader(actual_type)?;
    state.gl.shader_source(shader, &merged_source);
    state.gl.compile_shader(shader);

    let status = state.gl.get_shader_compile_status(shader);
    let compilation_message = state.gl.get_shader_info_log(shader);

    if !status {
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

fn prepare_source_code(code: &str) -> String {
    let mut shared = "\n// include 'shared.glsl'\n".to_owned();
    shared += include_str!("../shaders/shared.glsl");
    shared += "\n// end of include\n";

    let code = if let Some(p) = code.find('#') {
        let mut full = code.to_owned();
        let end = p + full[p..].find('\n').unwrap() + 1;
        full.insert_str(end, &shared);
        full
    } else {
        shared += code;
        shared
    };

    // HACK
    #[cfg(target_arch = "wasm32")]
    {
        code.replace("#version 330 core", "#version 300 es")
    }

    #[cfg(not(target_arch = "wasm32"))]
    code
}

impl GpuProgram {
    pub fn from_source(
        state: &mut PipelineState,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<GpuProgram, RendererError> {
        unsafe {
            let vertex_shader = create_shader(
                state,
                format!("{}_VertexShader", name),
                glow::VERTEX_SHADER,
                vertex_source,
            )?;
            let fragment_shader = create_shader(
                state,
                format!("{}_FragmentShader", name),
                glow::FRAGMENT_SHADER,
                fragment_source,
            )?;
            let program = state.gl.create_program()?;
            state.gl.attach_shader(program, vertex_shader);
            state.gl.delete_shader(vertex_shader);
            state.gl.attach_shader(program, fragment_shader);
            state.gl.delete_shader(fragment_shader);
            state.gl.link_program(program);
            let status = state.gl.get_program_link_status(program);
            let link_message = state.gl.get_program_info_log(program);

            if !status {
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
                    state,
                    id: program,
                    thread_mark: PhantomData,
                })
            }
        }
    }

    pub fn uniform_location(
        &self,
        state: &mut PipelineState,
        name: &str,
    ) -> Result<UniformLocation, RendererError> {
        unsafe {
            if let Some(id) = state.gl.get_uniform_location(self.id, name) {
                Ok(UniformLocation {
                    id,
                    thread_mark: PhantomData,
                })
            } else {
                Err(RendererError::UnableToFindShaderUniform(name.to_owned()))
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
        scope_profile!();

        state.set_program(self.id);

        let location = Some(&location.id);
        unsafe {
            match value {
                UniformValue::Sampler { index, texture } => {
                    state.gl.uniform_1_i32(location, *index as i32);
                    texture.borrow().bind(state, *index);
                }
                UniformValue::Bool(value) => {
                    state.gl.uniform_1_i32(
                        location,
                        if *value { glow::TRUE } else { glow::FALSE } as i32,
                    );
                }
                UniformValue::Integer(value) => {
                    state.gl.uniform_1_i32(location, *value);
                }
                UniformValue::Float(value) => {
                    state.gl.uniform_1_f32(location, *value);
                }
                UniformValue::Vector2(value) => {
                    state.gl.uniform_2_f32(location, value.x, value.y);
                }
                UniformValue::Vector3(value) => {
                    state.gl.uniform_3_f32(location, value.x, value.y, value.z);
                }
                UniformValue::Vector4(value) => {
                    state
                        .gl
                        .uniform_4_f32(location, value.x, value.y, value.z, value.w);
                }
                UniformValue::IntegerArray(value) => {
                    state.gl.uniform_1_i32_slice(location, value);
                }
                UniformValue::FloatArray(value) => {
                    state.gl.uniform_1_f32_slice(location, value);
                }
                UniformValue::Vec2Array(value) => {
                    state.gl.uniform_2_f32_slice(
                        location,
                        std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 2),
                    );
                }
                UniformValue::Vec3Array(value) => {
                    state.gl.uniform_3_f32_slice(
                        location,
                        std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 3),
                    );
                }
                UniformValue::Vec4Array(value) => {
                    state.gl.uniform_4_f32_slice(
                        location,
                        std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 4),
                    );
                }
                UniformValue::Matrix4(value) => {
                    state
                        .gl
                        .uniform_matrix_4_f32_slice(location, false, value.as_slice());
                }
                UniformValue::Matrix3(value) => {
                    state
                        .gl
                        .uniform_matrix_3_f32_slice(location, false, value.as_slice());
                }
                UniformValue::Mat4Array(value) => {
                    state.gl.uniform_matrix_4_f32_slice(
                        location,
                        false,
                        std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 16),
                    );
                }
                UniformValue::Color(value) => {
                    let rgba = value.as_frgba();
                    state
                        .gl
                        .uniform_4_f32(location, rgba.x, rgba.y, rgba.z, rgba.w);
                }
            }
        }
    }
}

impl Drop for GpuProgram {
    fn drop(&mut self) {
        unsafe {
            (*self.state).gl.delete_program(self.id);
        }
    }
}
