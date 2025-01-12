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
        log::{Log, MessageKind},
        ImmutableString,
    },
    error::FrameworkError,
    gl::server::{GlGraphicsServer, GlKind},
    gpu_program::{
        GpuProgram, SamplerKind, ShaderPropertyKind, ShaderResourceDefinition, ShaderResourceKind,
        UniformLocation,
    },
};
use fxhash::FxHashMap;
use glow::HasContext;
use std::{
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
            format!("Failed to compile {name} shader: {compilation_message}"),
        );
        Err(FrameworkError::ShaderCompilationFailed {
            shader_name: name,
            error_message: compilation_message,
        })
    } else {
        let msg = if compilation_message.is_empty()
            || compilation_message.chars().all(|c| c.is_whitespace())
        {
            format!("Shader {name} compiled successfully!")
        } else {
            format!("Shader {name} compiled successfully!\nAdditional info: {compilation_message}")
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

impl GlProgram {
    pub fn from_source_and_properties(
        server: &GlGraphicsServer,
        program_name: &str,
        vertex_source: &str,
        fragment_source: &str,
        resources: &[ShaderResourceDefinition],
    ) -> Result<GlProgram, FrameworkError> {
        let mut vertex_source = vertex_source.to_string();
        let mut fragment_source = fragment_source.to_string();

        // Initial validation. The program will be validated once more by the compiler.
        for resource in resources {
            for other_resource in resources {
                if std::ptr::eq(resource, other_resource) {
                    continue;
                }

                if std::mem::discriminant(&resource.kind)
                    == std::mem::discriminant(&other_resource.kind)
                {
                    if resource.binding == other_resource.binding {
                        return Err(FrameworkError::Custom(format!(
                            "Resource {} and {} using the same binding point {} \
                            in the {program_name} GPU program.",
                            resource.name, other_resource.name, resource.binding
                        )));
                    }

                    if resource.name == other_resource.name {
                        return Err(FrameworkError::Custom(format!(
                            "There are two or more resources with same name {} \
                                in the {program_name} GPU program.",
                            resource.name
                        )));
                    }
                }
            }
        }

        // Generate appropriate texture binding points and uniform blocks for the specified properties.
        for initial_source in [&mut vertex_source, &mut fragment_source] {
            let mut texture_bindings = String::new();

            for property in resources {
                let resource_name = &property.name;
                match property.kind {
                    ShaderResourceKind::Texture { kind, .. } => {
                        let glsl_name = kind.glsl_name();
                        texture_bindings += &format!("uniform {glsl_name} {resource_name};\n");
                    }
                    ShaderResourceKind::PropertyGroup(ref fields) => {
                        if fields.is_empty() {
                            Log::warn(format!(
                                "Uniform block {resource_name} is empty and will be ignored!"
                            ));
                            continue;
                        }
                        let mut block = format!("struct T{resource_name}{{\n");
                        for field in fields {
                            let field_name = &field.name;
                            match field.kind {
                                ShaderPropertyKind::Float(_) => {
                                    block += &format!("\tfloat {field_name};\n");
                                }
                                ShaderPropertyKind::FloatArray { max_len, .. } => {
                                    block += &format!("\tfloat {field_name}[{max_len}];\n");
                                }
                                ShaderPropertyKind::Int(_) => {
                                    block += &format!("\tint {field_name};\n");
                                }
                                ShaderPropertyKind::IntArray { max_len, .. } => {
                                    block += &format!("\tint {field_name}[{max_len}];\n");
                                }
                                ShaderPropertyKind::UInt(_) => {
                                    block += &format!("\tuint {field_name};\n");
                                }
                                ShaderPropertyKind::UIntArray { max_len, .. } => {
                                    block += &format!("\tuint {field_name}[{max_len}];\n");
                                }
                                ShaderPropertyKind::Bool(_) => {
                                    block += &format!("\tbool {field_name};\n");
                                }
                                ShaderPropertyKind::Vector2(_) => {
                                    block += &format!("\tvec2 {field_name};\n");
                                }
                                ShaderPropertyKind::Vector2Array { max_len, .. } => {
                                    block += &format!("\tvec2 {field_name}[{max_len}];\n");
                                }
                                ShaderPropertyKind::Vector3(_) => {
                                    block += &format!("\tvec3 {field_name};\n");
                                }
                                ShaderPropertyKind::Vector3Array { max_len, .. } => {
                                    block += &format!("\tvec3 {field_name}[{max_len}];\n");
                                }
                                ShaderPropertyKind::Vector4(_) => {
                                    block += &format!("\tvec4 {field_name};\n");
                                }
                                ShaderPropertyKind::Vector4Array { max_len, .. } => {
                                    block += &format!("\tvec4 {field_name}[{max_len}];\n");
                                }
                                ShaderPropertyKind::Matrix2(_) => {
                                    block += &format!("\tmat2 {field_name};\n");
                                }
                                ShaderPropertyKind::Matrix2Array { max_len, .. } => {
                                    block += &format!("\tmat2 {field_name}[{max_len}];\n");
                                }
                                ShaderPropertyKind::Matrix3(_) => {
                                    block += &format!("\tmat3 {field_name};\n");
                                }
                                ShaderPropertyKind::Matrix3Array { max_len, .. } => {
                                    block += &format!("\tmat3 {field_name}[{max_len}];\n");
                                }
                                ShaderPropertyKind::Matrix4(_) => {
                                    block += &format!("\tmat4 {field_name};\n");
                                }
                                ShaderPropertyKind::Matrix4Array { max_len, .. } => {
                                    block += &format!("\tmat4 {field_name}[{max_len}];\n");
                                }
                                ShaderPropertyKind::Color { .. } => {
                                    block += &format!("\tvec4 {field_name};\n");
                                }
                            }
                        }
                        block += "};\n";
                        block += &format!("layout(std140) uniform U{resource_name} {{ T{resource_name} {resource_name}; }};\n");
                        initial_source.insert_str(0, &block);
                    }
                }
            }
            initial_source.insert_str(0, &texture_bindings);
        }

        let program = Self::from_source(server, program_name, &vertex_source, &fragment_source)?;

        unsafe {
            server.set_program(Some(program.id));
            for resource_definition in resources {
                match resource_definition.kind {
                    ShaderResourceKind::Texture { .. } => {
                        if let Some(location) = server
                            .gl
                            .get_uniform_location(program.id, &resource_definition.name)
                        {
                            server
                                .gl
                                .uniform_1_i32(Some(&location), resource_definition.binding as i32);
                        }
                    }
                    ShaderResourceKind::PropertyGroup { .. } => {
                        if let Some(shader_block_index) = server.gl.get_uniform_block_index(
                            program.id,
                            &format!("U{}", resource_definition.name),
                        ) {
                            server.gl.uniform_block_binding(
                                program.id,
                                shader_block_index,
                                resource_definition.binding as u32,
                            )
                        } else {
                            Log::warn(format!(
                                "Couldn't find uniform block U{}",
                                resource_definition.name
                            ));
                        }
                    }
                }
            }
        }

        Ok(program)
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
                format!("{name}_VertexShader"),
                glow::VERTEX_SHADER,
                vertex_source,
                server.gl_kind(),
            )?;
            let fragment_shader = create_shader(
                server,
                format!("{name}_FragmentShader"),
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
                    format!("Failed to link {name} shader: {link_message}"),
                );
                Err(FrameworkError::ShaderLinkingFailed {
                    shader_name: name.to_owned(),
                    error_message: link_message,
                })
            } else {
                let msg = if link_message.is_empty()
                    || link_message.chars().all(|c| c.is_whitespace())
                {
                    format!("Shader {name} linked successfully!")
                } else {
                    format!("Shader {name} linked successfully!\nAdditional info: {link_message}")
                };

                Log::writeln(MessageKind::Information, msg);

                Ok(Self {
                    state: server.weak(),
                    id: program,
                    thread_mark: PhantomData,
                    uniform_locations: Default::default(),
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
