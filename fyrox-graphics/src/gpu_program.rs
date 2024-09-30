// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    core::{
        algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
        color::Color,
        log::{Log, MessageKind},
        sstorage::ImmutableString,
    },
    error::FrameworkError,
    state::{GlGraphicsServer, GlKind},
};
use fxhash::FxHashMap;
use glow::HasContext;
use std::{cell::RefCell, marker::PhantomData, ops::Deref, rc::Weak};

pub struct GpuProgram {
    state: Weak<GlGraphicsServer>,
    pub id: glow::Program,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
    uniform_locations: RefCell<FxHashMap<ImmutableString, Option<UniformLocation>>>,
    pub built_in_uniform_locations: [Option<UniformLocation>; BuiltInUniform::Count as usize],
    pub built_in_uniform_blocks: [Option<u32>; BuiltInUniformBlock::Count as usize],
}

#[repr(usize)]
pub enum BuiltInUniformBlock {
    BoneMatrices,
    InstanceData,
    Count,
}

#[repr(usize)]
pub enum BuiltInUniform {
    WorldMatrix,
    ViewProjectionMatrix,
    WorldViewProjectionMatrix,
    UseSkeletalAnimation,
    CameraPosition,
    CameraUpVector,
    CameraSideVector,
    ZNear,
    ZFar,
    SceneDepth,
    UsePOM,
    LightPosition,
    BlendShapesStorage,
    BlendShapesWeights,
    BlendShapesCount,
    LightCount,
    LightsColorRadius,
    LightsPosition,
    LightsDirection,
    LightsParameters,
    AmbientLight,
    // Must be last.
    Count,
}

#[derive(Clone, Debug)]
pub struct UniformLocation {
    pub id: glow::UniformLocation,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

unsafe fn create_shader(
    server: &GlGraphicsServer,
    name: String,
    actual_type: u32,
    source: &str,
    gl_kind: GlKind,
) -> Result<glow::Shader, FrameworkError> {
    let merged_source = prepare_source_code(source, gl_kind);

    let shader = server.gl.create_shader(actual_type)?;
    server.gl.shader_source(shader, &merged_source);
    server.gl.compile_shader(shader);

    let status = server.gl.get_shader_compile_status(shader);
    let compilation_message = server.gl.get_shader_info_log(shader);

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
        let msg = if compilation_message.is_empty()
            || compilation_message.chars().all(|c| c.is_whitespace())
        {
            format!("Shader {} compiled successfully!", name)
        } else {
            format!(
                "Shader {} compiled successfully!\nAdditional info: {}",
                name, compilation_message
            )
        };

        Log::writeln(MessageKind::Information, msg);

        Ok(shader)
    }
}

fn prepare_source_code(code: &str, gl_kind: GlKind) -> String {
    let mut full_source_code = "#version 330 core\n// include 'shared.glsl'\n".to_owned();

    if gl_kind == GlKind::OpenGLES {
        full_source_code += r#"    
            precision highp float;
            precision lowp usampler2D;
            precision lowp sampler3D;
        "#;
    }

    full_source_code += include_str!("shaders/shared.glsl");
    full_source_code += "\n// end of include\n";
    full_source_code += code;

    if gl_kind == GlKind::OpenGLES {
        full_source_code.replace("#version 330 core", "#version 300 es")
    } else {
        full_source_code
    }
}

pub struct GpuProgramBinding<'a, 'b> {
    pub state: &'a GlGraphicsServer,
    active_sampler: u32,
    pub program: &'b GpuProgram,
}

impl<'a, 'b> GpuProgramBinding<'a, 'b> {
    pub fn uniform_location(&self, name: &ImmutableString) -> Option<UniformLocation> {
        self.program.uniform_location_internal(self.state, name)
    }

    pub fn active_sampler(&self) -> u32 {
        self.active_sampler
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
            if !value.is_empty() {
                self.state.gl.uniform_1_i32_slice(Some(&location.id), value);
            }
        }
        self
    }

    #[inline(always)]
    pub fn set_u32_slice(&mut self, location: &UniformLocation, value: &[u32]) -> &mut Self {
        unsafe {
            if !value.is_empty() {
                self.state.gl.uniform_1_u32_slice(Some(&location.id), value);
            }
        }
        self
    }

    #[inline(always)]
    pub fn set_f32_slice(&mut self, location: &UniformLocation, value: &[f32]) -> &mut Self {
        unsafe {
            if !value.is_empty() {
                self.state.gl.uniform_1_f32_slice(Some(&location.id), value);
            }
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
            if !value.is_empty() {
                self.state.gl.uniform_2_f32_slice(
                    Some(&location.id),
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 2),
                );
            }
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
            if !value.is_empty() {
                self.state.gl.uniform_3_f32_slice(
                    Some(&location.id),
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 3),
                );
            }
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
            if !value.is_empty() {
                self.state.gl.uniform_4_f32_slice(
                    Some(&location.id),
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 4),
                );
            }
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
            if !value.is_empty() {
                self.state.gl.uniform_matrix_2_f32_slice(
                    Some(&location.id),
                    false,
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 4),
                );
            }
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
            if !value.is_empty() {
                self.state.gl.uniform_matrix_3_f32_slice(
                    Some(&location.id),
                    false,
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 9),
                );
            }
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
            if !value.is_empty() {
                self.state.gl.uniform_matrix_4_f32_slice(
                    Some(&location.id),
                    false,
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 16),
                );
            }
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

#[inline]
fn fetch_uniform_location(
    server: &GlGraphicsServer,
    program: glow::Program,
    id: &str,
) -> Option<UniformLocation> {
    unsafe {
        server
            .gl
            .get_uniform_location(program, id)
            .map(|id| UniformLocation {
                id,
                thread_mark: PhantomData,
            })
    }
}

pub fn fetch_uniform_block_index(
    server: &GlGraphicsServer,
    program: glow::Program,
    name: &str,
) -> Option<u32> {
    unsafe { server.gl.get_uniform_block_index(program, name) }
}

fn fetch_built_in_uniform_blocks(
    server: &GlGraphicsServer,
    program: glow::Program,
) -> [Option<u32>; BuiltInUniformBlock::Count as usize] {
    let mut locations = [None; BuiltInUniformBlock::Count as usize];
    locations[BuiltInUniformBlock::BoneMatrices as usize] =
        fetch_uniform_block_index(server, program, "FyroxBoneMatrices");
    locations[BuiltInUniformBlock::InstanceData as usize] =
        fetch_uniform_block_index(server, program, "FyroxInstanceData");
    locations
}

fn fetch_built_in_uniform_locations(
    server: &GlGraphicsServer,
    program: glow::Program,
) -> [Option<UniformLocation>; BuiltInUniform::Count as usize] {
    const INIT: Option<UniformLocation> = None;
    let mut locations = [INIT; BuiltInUniform::Count as usize];

    locations[BuiltInUniform::WorldMatrix as usize] =
        fetch_uniform_location(server, program, "fyrox_worldMatrix");
    locations[BuiltInUniform::ViewProjectionMatrix as usize] =
        fetch_uniform_location(server, program, "fyrox_viewProjectionMatrix");
    locations[BuiltInUniform::WorldViewProjectionMatrix as usize] =
        fetch_uniform_location(server, program, "fyrox_worldViewProjection");

    locations[BuiltInUniform::UseSkeletalAnimation as usize] =
        fetch_uniform_location(server, program, "fyrox_useSkeletalAnimation");

    locations[BuiltInUniform::CameraPosition as usize] =
        fetch_uniform_location(server, program, "fyrox_cameraPosition");
    locations[BuiltInUniform::CameraUpVector as usize] =
        fetch_uniform_location(server, program, "fyrox_cameraUpVector");
    locations[BuiltInUniform::CameraSideVector as usize] =
        fetch_uniform_location(server, program, "fyrox_cameraSideVector");
    locations[BuiltInUniform::ZNear as usize] =
        fetch_uniform_location(server, program, "fyrox_zNear");
    locations[BuiltInUniform::ZFar as usize] =
        fetch_uniform_location(server, program, "fyrox_zFar");

    locations[BuiltInUniform::SceneDepth as usize] =
        fetch_uniform_location(server, program, "fyrox_sceneDepth");

    locations[BuiltInUniform::UsePOM as usize] =
        fetch_uniform_location(server, program, "fyrox_usePOM");

    locations[BuiltInUniform::BlendShapesStorage as usize] =
        fetch_uniform_location(server, program, "fyrox_blendShapesStorage");
    locations[BuiltInUniform::BlendShapesWeights as usize] =
        fetch_uniform_location(server, program, "fyrox_blendShapesWeights");
    locations[BuiltInUniform::BlendShapesCount as usize] =
        fetch_uniform_location(server, program, "fyrox_blendShapesCount");

    locations[BuiltInUniform::LightCount as usize] =
        fetch_uniform_location(server, program, "fyrox_lightCount");
    locations[BuiltInUniform::LightsColorRadius as usize] =
        fetch_uniform_location(server, program, "fyrox_lightsColorRadius");
    locations[BuiltInUniform::LightsPosition as usize] =
        fetch_uniform_location(server, program, "fyrox_lightsPosition");
    locations[BuiltInUniform::LightsDirection as usize] =
        fetch_uniform_location(server, program, "fyrox_lightsDirection");
    locations[BuiltInUniform::LightsParameters as usize] =
        fetch_uniform_location(server, program, "fyrox_lightsParameters");
    locations[BuiltInUniform::AmbientLight as usize] =
        fetch_uniform_location(server, program, "fyrox_ambientLightColor");
    locations[BuiltInUniform::LightPosition as usize] =
        fetch_uniform_location(server, program, "fyrox_lightPosition");

    locations
}

impl GpuProgram {
    pub fn from_source(
        server: &GlGraphicsServer,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<GpuProgram, FrameworkError> {
        unsafe {
            let vertex_shader = create_shader(
                server,
                format!("{}_VertexShader", name),
                glow::VERTEX_SHADER,
                vertex_source,
                server.gl_kind(),
            )?;
            let fragment_shader = create_shader(
                server,
                format!("{}_FragmentShader", name),
                glow::FRAGMENT_SHADER,
                fragment_source,
                server.gl_kind(),
            )?;
            let program = server.gl.create_program()?;
            server.gl.attach_shader(program, vertex_shader);
            server.gl.delete_shader(vertex_shader);
            server.gl.attach_shader(program, fragment_shader);
            server.gl.delete_shader(fragment_shader);
            server.gl.link_program(program);
            let status = server.gl.get_program_link_status(program);
            let link_message = server.gl.get_program_info_log(program);

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
                let msg =
                    if link_message.is_empty() || link_message.chars().all(|c| c.is_whitespace()) {
                        format!("Shader {} linked successfully!", name)
                    } else {
                        format!(
                            "Shader {} linked successfully!\nAdditional info: {}",
                            name, link_message
                        )
                    };

                Log::writeln(MessageKind::Information, msg);

                Ok(Self {
                    state: server.weak(),
                    id: program,
                    thread_mark: PhantomData,
                    uniform_locations: Default::default(),
                    built_in_uniform_locations: fetch_built_in_uniform_locations(server, program),
                    built_in_uniform_blocks: fetch_built_in_uniform_blocks(server, program),
                })
            }
        }
    }

    pub fn uniform_location_internal(
        &self,
        server: &GlGraphicsServer,
        name: &ImmutableString,
    ) -> Option<UniformLocation> {
        let mut locations = self.uniform_locations.borrow_mut();

        if let Some(cached_location) = locations.get(name) {
            cached_location.clone()
        } else {
            let location = fetch_uniform_location(server, self.id, name.deref());

            locations.insert(name.clone(), location.clone());

            location
        }
    }

    pub fn uniform_location(
        &self,
        server: &GlGraphicsServer,
        name: &ImmutableString,
    ) -> Result<UniformLocation, FrameworkError> {
        self.uniform_location_internal(server, name)
            .ok_or_else(|| FrameworkError::UnableToFindShaderUniform(name.deref().to_owned()))
    }

    pub fn uniform_block_index(
        &self,
        server: &GlGraphicsServer,
        name: &ImmutableString,
    ) -> Result<usize, FrameworkError> {
        unsafe {
            server
                .gl
                .get_uniform_block_index(self.id, name)
                .map(|index| index as usize)
                .ok_or_else(|| {
                    FrameworkError::UnableToFindShaderUniformBlock(name.deref().to_owned())
                })
        }
    }

    pub fn bind<'a, 'b>(&'b self, state: &'a GlGraphicsServer) -> GpuProgramBinding<'a, 'b> {
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
        if let Some(state) = self.state.upgrade() {
            unsafe {
                state.gl.delete_program(self.id);
            }
        }
    }
}
