use crate::core::sstorage::ImmutableString;
use crate::{
    core::{
        algebra::Matrix2,
        algebra::{Matrix3, Matrix4, Vector2, Vector3, Vector4},
        color::Color,
    },
    renderer::framework::{error::FrameworkError, gpu_texture::GpuTexture, state::PipelineState},
    utils::log::{Log, MessageKind},
};
use fxhash::FxHashMap;
use glow::HasContext;
use std::ops::Deref;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

pub struct GpuProgram {
    state: *mut PipelineState,
    id: glow::Program,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
    uniform_locations: RefCell<FxHashMap<ImmutableString, Option<UniformLocation>>>,
    pub(crate) built_in_uniform_locations:
        [Option<UniformLocation>; BuiltInUniform::Count as usize],
}

#[repr(usize)]
pub enum BuiltInUniform {
    WorldMatrix,
    WorldViewProjectionMatrix,
    BoneMatrices,
    UseSkeletalAnimation,
    CameraPosition,
    UsePOM,
    LightPosition,
    // Must be last.
    Count,
}

#[derive(Clone, Debug)]
pub struct UniformLocation {
    id: glow::UniformLocation,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

unsafe fn create_shader(
    state: &mut PipelineState,
    name: String,
    actual_type: u32,
    source: &str,
) -> Result<glow::Shader, FrameworkError> {
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
        Err(FrameworkError::ShaderCompilationFailed {
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

#[allow(clippy::let_and_return)]
fn prepare_source_code(code: &str) -> String {
    let mut full_source_code = "#version 330 core\n// include 'shared.glsl'\n".to_owned();

    // HACK
    #[cfg(target_arch = "wasm32")]
    {
        full_source_code += r#"    
            precision highp float;
            precision lowp usampler2D;
            precision lowp sampler3D;
        "#;
    }

    full_source_code += include_str!("shaders/shared.glsl");
    full_source_code += "\n// end of include\n";
    full_source_code += code;

    // HACK
    #[cfg(target_arch = "wasm32")]
    {
        full_source_code.replace("#version 330 core", "#version 300 es")
    }

    #[cfg(not(target_arch = "wasm32"))]
    full_source_code
}

pub struct GpuProgramBinding<'a, 'b> {
    pub state: &'a mut PipelineState,
    active_sampler: u32,
    pub(crate) program: &'b GpuProgram,
}

impl<'a, 'b> GpuProgramBinding<'a, 'b> {
    pub fn uniform_location(&self, name: &ImmutableString) -> Option<UniformLocation> {
        self.program.uniform_location_internal(self.state, name)
    }

    #[inline(always)]
    pub fn set_texture(
        &mut self,
        location: &UniformLocation,
        texture: &Rc<RefCell<GpuTexture>>,
    ) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_1_i32(Some(&location.id), self.active_sampler as i32)
        };
        texture.borrow().bind(self.state, self.active_sampler);
        self.active_sampler += 1;
        self
    }

    #[inline(always)]
    pub fn set_bool(&mut self, location: &UniformLocation, value: bool) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_i32(
                Some(&location.id),
                if value { glow::TRUE } else { glow::FALSE } as i32,
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_i32(&mut self, location: &UniformLocation, value: i32) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_i32(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_u32(&mut self, location: &UniformLocation, value: u32) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_u32(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_f32(&mut self, location: &UniformLocation, value: f32) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_f32(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_vector2(&mut self, location: &UniformLocation, value: &Vector2<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_2_f32(Some(&location.id), value.x, value.y);
        }
        self
    }

    #[inline(always)]
    pub fn set_vector3(&mut self, location: &UniformLocation, value: &Vector3<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_3_f32(Some(&location.id), value.x, value.y, value.z);
        }
        self
    }

    #[inline(always)]
    pub fn set_vector4(&mut self, location: &UniformLocation, value: &Vector4<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_4_f32(Some(&location.id), value.x, value.y, value.z, value.w);
        }
        self
    }

    #[inline(always)]
    pub fn set_i32_slice(&mut self, location: &UniformLocation, value: &[i32]) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_i32_slice(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_u32_slice(&mut self, location: &UniformLocation, value: &[u32]) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_u32_slice(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_f32_slice(&mut self, location: &UniformLocation, value: &[f32]) -> &mut Self {
        unsafe {
            self.state.gl.uniform_1_f32_slice(Some(&location.id), value);
        }
        self
    }

    #[inline(always)]
    pub fn set_vector2_slice(
        &mut self,
        location: &UniformLocation,
        value: &[Vector2<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_2_f32_slice(
                Some(&location.id),
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 2),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_vector3_slice(
        &mut self,
        location: &UniformLocation,
        value: &[Vector3<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_3_f32_slice(
                Some(&location.id),
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 3),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_vector4_slice(
        &mut self,
        location: &UniformLocation,
        value: &[Vector4<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_4_f32_slice(
                Some(&location.id),
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 4),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix2(&mut self, location: &UniformLocation, value: &Matrix2<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_matrix_2_f32_slice(Some(&location.id), false, value.as_slice());
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix2_array(
        &mut self,
        location: &UniformLocation,
        value: &[Matrix2<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_matrix_2_f32_slice(
                Some(&location.id),
                false,
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 4),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix3(&mut self, location: &UniformLocation, value: &Matrix3<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_matrix_3_f32_slice(Some(&location.id), false, value.as_slice());
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix3_array(
        &mut self,
        location: &UniformLocation,
        value: &[Matrix3<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_matrix_3_f32_slice(
                Some(&location.id),
                false,
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 9),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix4(&mut self, location: &UniformLocation, value: &Matrix4<f32>) -> &mut Self {
        unsafe {
            self.state
                .gl
                .uniform_matrix_4_f32_slice(Some(&location.id), false, value.as_slice());
        }
        self
    }

    #[inline(always)]
    pub fn set_matrix4_array(
        &mut self,
        location: &UniformLocation,
        value: &[Matrix4<f32>],
    ) -> &mut Self {
        unsafe {
            self.state.gl.uniform_matrix_4_f32_slice(
                Some(&location.id),
                false,
                std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 16),
            );
        }
        self
    }

    #[inline(always)]
    pub fn set_linear_color(&mut self, location: &UniformLocation, value: &Color) -> &mut Self {
        unsafe {
            let srgb_a = value.srgb_to_linear_f32();
            self.state
                .gl
                .uniform_4_f32(Some(&location.id), srgb_a.x, srgb_a.y, srgb_a.z, srgb_a.w);
        }
        self
    }

    #[inline(always)]
    pub fn set_srgb_color(&mut self, location: &UniformLocation, value: &Color) -> &mut Self {
        unsafe {
            let rgba = value.as_frgba();
            self.state
                .gl
                .uniform_4_f32(Some(&location.id), rgba.x, rgba.y, rgba.z, rgba.w);
        }
        self
    }
}

fn fetch_uniform_location(
    state: &PipelineState,
    program: glow::Program,
    id: &str,
) -> Option<UniformLocation> {
    unsafe {
        state
            .gl
            .get_uniform_location(program, id)
            .map(|id| UniformLocation {
                id,
                thread_mark: PhantomData,
            })
    }
}

fn fetch_built_in_uniform_locations(
    state: &PipelineState,
    program: glow::Program,
) -> [Option<UniformLocation>; BuiltInUniform::Count as usize] {
    const INIT: Option<UniformLocation> = None;
    let mut locations = [INIT; BuiltInUniform::Count as usize];

    locations[BuiltInUniform::WorldMatrix as usize] =
        fetch_uniform_location(state, program, "rg3d_worldMatrix");
    locations[BuiltInUniform::WorldViewProjectionMatrix as usize] =
        fetch_uniform_location(state, program, "rg3d_worldViewProjection");
    locations[BuiltInUniform::BoneMatrices as usize] =
        fetch_uniform_location(state, program, "rg3d_boneMatrices");
    locations[BuiltInUniform::UseSkeletalAnimation as usize] =
        fetch_uniform_location(state, program, "rg3d_useSkeletalAnimation");
    locations[BuiltInUniform::CameraPosition as usize] =
        fetch_uniform_location(state, program, "rg3d_cameraPosition");
    locations[BuiltInUniform::UsePOM as usize] =
        fetch_uniform_location(state, program, "rg3d_usePOM");
    locations[BuiltInUniform::LightPosition as usize] =
        fetch_uniform_location(state, program, "rg3d_lightPosition");

    locations
}

impl GpuProgram {
    pub fn from_source(
        state: &mut PipelineState,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<GpuProgram, FrameworkError> {
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
                Err(FrameworkError::ShaderLinkingFailed {
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
                    uniform_locations: Default::default(),
                    built_in_uniform_locations: fetch_built_in_uniform_locations(state, program),
                })
            }
        }
    }

    pub fn uniform_location_internal(
        &self,
        state: &PipelineState,
        name: &ImmutableString,
    ) -> Option<UniformLocation> {
        let mut locations = self.uniform_locations.borrow_mut();

        if let Some(cached_location) = locations.get(name) {
            cached_location.clone()
        } else {
            let location = fetch_uniform_location(state, self.id, name.deref());

            locations.insert(name.clone(), location.clone());

            location
        }
    }

    pub fn uniform_location(
        &self,
        state: &PipelineState,
        name: &ImmutableString,
    ) -> Result<UniformLocation, FrameworkError> {
        self.uniform_location_internal(state, name)
            .ok_or_else(|| FrameworkError::UnableToFindShaderUniform(name.deref().to_owned()))
    }

    pub fn bind<'a, 'b>(&'b self, state: &'a mut PipelineState) -> GpuProgramBinding<'a, 'b> {
        state.set_program(Some(self.id));
        GpuProgramBinding {
            state,
            active_sampler: 0,
            program: self,
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
