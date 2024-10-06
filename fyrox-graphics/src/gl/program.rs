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

use crate::gpu_program::SamplerKind;
use crate::{
    core::{
        algebra::{Matrix2, Matrix3, Matrix4, Vector2, Vector3, Vector4},
        color::Color,
        log::{Log, MessageKind},
        ImmutableString,
    },
    error::FrameworkError,
    gl::server::{GlGraphicsServer, GlKind},
    gpu_program::{
        BuiltInUniform, BuiltInUniformBlock, GpuProgram, PropertyDefinition, PropertyKind,
        UniformLocation,
    },
};
use fxhash::FxHashMap;
use glow::HasContext;
use std::{
    any::Any,
    cell::RefCell,
    marker::PhantomData,
    ops::Deref,
    rc::{Rc, Weak},
};

impl SamplerKind {
    pub fn glsl_name(&self) -> &str {
        match self {
            SamplerKind::Sampler1D => "sampler1D",
            SamplerKind::Sampler2D => "sampler2D",
            SamplerKind::Sampler3D => "sampler3D",
            SamplerKind::SamplerCube => "samplerCube",
            SamplerKind::USampler1D => "usampler1D",
            SamplerKind::USampler2D => "usampler2D",
            SamplerKind::USampler3D => "usampler3D",
            SamplerKind::USamplerCube => "usamplerCube",
        }
    }
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
            precision highp int;
            precision highp usampler2D;
            precision highp usampler3D;
            precision highp usamplerCube;
            precision highp sampler2D;
            precision highp sampler3D;
            precision highp samplerCube;
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

pub struct GlProgram {
    state: Weak<GlGraphicsServer>,
    pub id: glow::Program,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
    uniform_locations: RefCell<FxHashMap<ImmutableString, Option<UniformLocation>>>,
    pub built_in_uniform_locations: [Option<UniformLocation>; BuiltInUniform::Count as usize],
    pub built_in_uniform_blocks: [Option<usize>; BuiltInUniformBlock::Count as usize],
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
) -> Option<usize> {
    unsafe {
        server
            .gl
            .get_uniform_block_index(program, name)
            .map(|index| index as usize)
    }
}

fn fetch_built_in_uniform_blocks(
    server: &GlGraphicsServer,
    program: glow::Program,
) -> [Option<usize>; BuiltInUniformBlock::Count as usize] {
    let mut locations = [None; BuiltInUniformBlock::Count as usize];
    locations[BuiltInUniformBlock::BoneMatrices as usize] =
        fetch_uniform_block_index(server, program, "FyroxBoneMatrices");
    locations[BuiltInUniformBlock::InstanceData as usize] =
        fetch_uniform_block_index(server, program, "FyroxInstanceData");
    locations[BuiltInUniformBlock::CameraData as usize] =
        fetch_uniform_block_index(server, program, "FyroxCameraData");
    locations[BuiltInUniformBlock::MaterialProperties as usize] =
        fetch_uniform_block_index(server, program, "FyroxMaterialProperties");
    locations
}

fn fetch_built_in_uniform_locations(
    server: &GlGraphicsServer,
    program: glow::Program,
) -> [Option<UniformLocation>; BuiltInUniform::Count as usize] {
    const INIT: Option<UniformLocation> = None;
    let mut locations = [INIT; BuiltInUniform::Count as usize];

    locations[BuiltInUniform::SceneDepth as usize] =
        fetch_uniform_location(server, program, "fyrox_sceneDepth");

    locations[BuiltInUniform::UsePOM as usize] =
        fetch_uniform_location(server, program, "fyrox_usePOM");

    locations[BuiltInUniform::BlendShapesStorage as usize] =
        fetch_uniform_location(server, program, "fyrox_blendShapesStorage");

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

impl GlProgram {
    pub fn from_source_and_properties(
        server: &GlGraphicsServer,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
        properties: &[PropertyDefinition],
    ) -> Result<GlProgram, FrameworkError> {
        let mut vertex_source = vertex_source.to_string();
        let mut fragment_source = fragment_source.to_string();

        // Generate appropriate texture binding points and uniform blocks for the specified properties.
        for initial_source in [&mut vertex_source, &mut fragment_source] {
            let mut texture_bindings = String::new();
            let mut uniform_block = "struct TProperties {\n".to_string();
            let mut sampler_count = 0;
            for property in properties {
                let name = &property.name;
                match property.kind {
                    PropertyKind::Float(_) => {
                        uniform_block += &format!("\tfloat {name};\n");
                    }
                    PropertyKind::FloatArray { max_len, .. } => {
                        uniform_block += &format!("\tfloat {name}[{max_len}];\n");
                    }
                    PropertyKind::Int(_) => {
                        uniform_block += &format!("\tint {name};\n");
                    }
                    PropertyKind::IntArray { max_len, .. } => {
                        uniform_block += &format!("\tint {name}[{max_len}];\n");
                    }
                    PropertyKind::UInt(_) => {
                        uniform_block += &format!("\tuint {name};\n");
                    }
                    PropertyKind::UIntArray { max_len, .. } => {
                        uniform_block += &format!("\tuint {name}[{max_len}];\n");
                    }
                    PropertyKind::Bool(_) => {
                        uniform_block += &format!("\tbool {name};\n");
                    }
                    PropertyKind::Vector2(_) => {
                        uniform_block += &format!("\tvec2 {name};\n");
                    }
                    PropertyKind::Vector2Array { max_len, .. } => {
                        uniform_block += &format!("\tvec2 {name}[{max_len}];\n");
                    }
                    PropertyKind::Vector3(_) => {
                        uniform_block += &format!("\tvec3 {name};\n");
                    }
                    PropertyKind::Vector3Array { max_len, .. } => {
                        uniform_block += &format!("\tvec3 {name}[{max_len}];\n");
                    }
                    PropertyKind::Vector4(_) => {
                        uniform_block += &format!("\tvec4 {name};\n");
                    }
                    PropertyKind::Vector4Array { max_len, .. } => {
                        uniform_block += &format!("\tvec4 {name}[{max_len}];\n");
                    }
                    PropertyKind::Matrix2(_) => {
                        uniform_block += &format!("\tmat2 {name};\n");
                    }
                    PropertyKind::Matrix2Array { max_len, .. } => {
                        uniform_block += &format!("\tmat2 {name}[{max_len}];\n");
                    }
                    PropertyKind::Matrix3(_) => {
                        uniform_block += &format!("\tmat3 {name};\n");
                    }
                    PropertyKind::Matrix3Array { max_len, .. } => {
                        uniform_block += &format!("\tmat3 {name}[{max_len}];\n");
                    }
                    PropertyKind::Matrix4(_) => {
                        uniform_block += &format!("\tmat4 {name};\n");
                    }
                    PropertyKind::Matrix4Array { max_len, .. } => {
                        uniform_block += &format!("\tmat4 {name}[{max_len}];\n");
                    }
                    PropertyKind::Color { .. } => {
                        uniform_block += &format!("\tvec4 {name};\n");
                    }
                    PropertyKind::Sampler { kind, .. } => {
                        let glsl_name = kind.glsl_name();
                        texture_bindings += &format!("uniform {glsl_name} {name};\n");
                        sampler_count += 1;
                    }
                }
            }

            uniform_block += "\n};\nlayout(std140) uniform FyroxMaterialProperties { TProperties properties; };\n";

            if (properties.len() - sampler_count) != 0 {
                initial_source.insert_str(0, &uniform_block);
            }
            initial_source.insert_str(0, &texture_bindings);
        }

        Self::from_source(server, name, &vertex_source, &fragment_source)
    }

    pub fn from_source(
        server: &GlGraphicsServer,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<GlProgram, FrameworkError> {
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

    pub fn uniform_location_internal(&self, name: &ImmutableString) -> Option<UniformLocation> {
        let mut locations = self.uniform_locations.borrow_mut();
        let server = self.state.upgrade().unwrap();
        if let Some(cached_location) = locations.get(name) {
            cached_location.clone()
        } else {
            let location = fetch_uniform_location(&server, self.id, name.deref());

            locations.insert(name.clone(), location.clone());

            location
        }
    }

    fn bind(&self) -> TempBinding {
        let server = self.state.upgrade().unwrap();
        server.set_program(Some(self.id));
        TempBinding { server }
    }
}

struct TempBinding {
    server: Rc<GlGraphicsServer>,
}

impl GpuProgram for GlProgram {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn built_in_uniform_locations(&self) -> &[Option<UniformLocation>] {
        &self.built_in_uniform_locations
    }

    fn built_in_uniform_blocks(&self) -> &[Option<usize>] {
        &self.built_in_uniform_blocks
    }

    fn uniform_location(&self, name: &ImmutableString) -> Result<UniformLocation, FrameworkError> {
        self.uniform_location_internal(name)
            .ok_or_else(|| FrameworkError::UnableToFindShaderUniform(name.deref().to_owned()))
    }

    fn uniform_block_index(&self, name: &ImmutableString) -> Result<usize, FrameworkError> {
        unsafe {
            self.bind()
                .server
                .gl
                .get_uniform_block_index(self.id, name)
                .map(|index| index as usize)
                .ok_or_else(|| {
                    FrameworkError::UnableToFindShaderUniformBlock(name.deref().to_owned())
                })
        }
    }

    #[inline(always)]
    fn set_bool(&self, location: &UniformLocation, value: bool) {
        unsafe {
            self.bind().server.gl.uniform_1_i32(
                Some(&location.id),
                if value { glow::TRUE } else { glow::FALSE } as i32,
            );
        }
    }

    #[inline(always)]
    fn set_i32(&self, location: &UniformLocation, value: i32) {
        unsafe {
            self.bind()
                .server
                .gl
                .uniform_1_i32(Some(&location.id), value);
        }
    }

    #[inline(always)]
    fn set_u32(&self, location: &UniformLocation, value: u32) {
        unsafe {
            self.bind()
                .server
                .gl
                .uniform_1_u32(Some(&location.id), value);
        }
    }

    #[inline(always)]
    fn set_f32(&self, location: &UniformLocation, value: f32) {
        unsafe {
            self.bind()
                .server
                .gl
                .uniform_1_f32(Some(&location.id), value);
        }
    }

    #[inline(always)]
    fn set_vector2(&self, location: &UniformLocation, value: &Vector2<f32>) {
        unsafe {
            self.bind()
                .server
                .gl
                .uniform_2_f32(Some(&location.id), value.x, value.y);
        }
    }

    #[inline(always)]
    fn set_vector3(&self, location: &UniformLocation, value: &Vector3<f32>) {
        unsafe {
            self.bind()
                .server
                .gl
                .uniform_3_f32(Some(&location.id), value.x, value.y, value.z);
        }
    }

    #[inline(always)]
    fn set_vector4(&self, location: &UniformLocation, value: &Vector4<f32>) {
        unsafe {
            self.bind().server.gl.uniform_4_f32(
                Some(&location.id),
                value.x,
                value.y,
                value.z,
                value.w,
            );
        }
    }

    #[inline(always)]
    fn set_i32_slice(&self, location: &UniformLocation, value: &[i32]) {
        unsafe {
            if !value.is_empty() {
                self.bind()
                    .server
                    .gl
                    .uniform_1_i32_slice(Some(&location.id), value);
            }
        }
    }

    #[inline(always)]
    fn set_u32_slice(&self, location: &UniformLocation, value: &[u32]) {
        unsafe {
            if !value.is_empty() {
                self.bind()
                    .server
                    .gl
                    .uniform_1_u32_slice(Some(&location.id), value);
            }
        }
    }

    #[inline(always)]
    fn set_f32_slice(&self, location: &UniformLocation, value: &[f32]) {
        unsafe {
            if !value.is_empty() {
                self.bind()
                    .server
                    .gl
                    .uniform_1_f32_slice(Some(&location.id), value);
            }
        }
    }

    #[inline(always)]
    fn set_vector2_slice(&self, location: &UniformLocation, value: &[Vector2<f32>]) {
        unsafe {
            if !value.is_empty() {
                self.bind().server.gl.uniform_2_f32_slice(
                    Some(&location.id),
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 2),
                );
            }
        }
    }

    #[inline(always)]
    fn set_vector3_slice(&self, location: &UniformLocation, value: &[Vector3<f32>]) {
        unsafe {
            if !value.is_empty() {
                self.bind().server.gl.uniform_3_f32_slice(
                    Some(&location.id),
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 3),
                );
            }
        }
    }

    #[inline(always)]
    fn set_vector4_slice(&self, location: &UniformLocation, value: &[Vector4<f32>]) {
        unsafe {
            if !value.is_empty() {
                self.bind().server.gl.uniform_4_f32_slice(
                    Some(&location.id),
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 4),
                );
            }
        }
    }

    #[inline(always)]
    fn set_matrix2(&self, location: &UniformLocation, value: &Matrix2<f32>) {
        unsafe {
            self.bind().server.gl.uniform_matrix_2_f32_slice(
                Some(&location.id),
                false,
                value.as_slice(),
            );
        }
    }

    #[inline(always)]
    fn set_matrix2_array(&self, location: &UniformLocation, value: &[Matrix2<f32>]) {
        unsafe {
            if !value.is_empty() {
                self.bind().server.gl.uniform_matrix_2_f32_slice(
                    Some(&location.id),
                    false,
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 4),
                );
            }
        }
    }

    #[inline(always)]
    fn set_matrix3(&self, location: &UniformLocation, value: &Matrix3<f32>) {
        unsafe {
            self.bind().server.gl.uniform_matrix_3_f32_slice(
                Some(&location.id),
                false,
                value.as_slice(),
            );
        }
    }

    #[inline(always)]
    fn set_matrix3_array(&self, location: &UniformLocation, value: &[Matrix3<f32>]) {
        unsafe {
            if !value.is_empty() {
                self.bind().server.gl.uniform_matrix_3_f32_slice(
                    Some(&location.id),
                    false,
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 9),
                );
            }
        }
    }

    #[inline(always)]
    fn set_matrix4(&self, location: &UniformLocation, value: &Matrix4<f32>) {
        unsafe {
            self.bind().server.gl.uniform_matrix_4_f32_slice(
                Some(&location.id),
                false,
                value.as_slice(),
            );
        }
    }

    #[inline(always)]
    fn set_matrix4_array(&self, location: &UniformLocation, value: &[Matrix4<f32>]) {
        unsafe {
            if !value.is_empty() {
                self.bind().server.gl.uniform_matrix_4_f32_slice(
                    Some(&location.id),
                    false,
                    std::slice::from_raw_parts(value.as_ptr() as *const f32, value.len() * 16),
                );
            }
        }
    }

    #[inline(always)]
    fn set_linear_color(&self, location: &UniformLocation, value: &Color) {
        unsafe {
            let srgb_a = value.srgb_to_linear_f32();
            self.bind().server.gl.uniform_4_f32(
                Some(&location.id),
                srgb_a.x,
                srgb_a.y,
                srgb_a.z,
                srgb_a.w,
            );
        }
    }

    #[inline(always)]
    fn set_srgb_color(&self, location: &UniformLocation, value: &Color) {
        unsafe {
            let rgba = value.as_frgba();
            self.bind()
                .server
                .gl
                .uniform_4_f32(Some(&location.id), rgba.x, rgba.y, rgba.z, rgba.w);
        }
    }
}

impl Drop for GlProgram {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            unsafe {
                state.gl.delete_program(self.id);
            }
        }
    }
}
