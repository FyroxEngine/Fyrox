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
    core::log::{Log, MessageKind},
    error::FrameworkError,
    gl::{
        server::{GlGraphicsServer, GlKind},
        ToGlConstant,
    },
    gpu_program::{
        GpuProgramTrait, GpuShaderTrait, SamplerKind, ShaderKind, ShaderPropertyKind,
        ShaderResourceDefinition, ShaderResourceKind,
    },
};
use glow::HasContext;
use std::{marker::PhantomData, ops::Range, rc::Weak};

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

fn count_lines(src: &str) -> isize {
    src.bytes().filter(|b| *b == b'\n').count() as isize
}

enum Vendor {
    Nvidia,
    Intel,
    Amd,
    Other,
}

impl Vendor {
    fn from_str(str: String) -> Self {
        if str.contains("nvidia") {
            Self::Nvidia
        } else if str.contains("amd") {
            Self::Amd
        } else if str.contains("intel") {
            Self::Intel
        } else {
            Self::Other
        }
    }

    fn regex(&self) -> regex::Regex {
        match self {
            Self::Nvidia => regex::Regex::new(r"\([0-9]*\)").unwrap(),
            Self::Intel | Vendor::Amd => regex::Regex::new(r"[0-9]*:").unwrap(),
            Self::Other => regex::Regex::new(r":[0-9]*").unwrap(),
        }
    }

    fn line_number_range(&self, match_range: regex::Match) -> Range<usize> {
        match self {
            Self::Nvidia => (match_range.start() + 1)..(match_range.end() - 1),
            Self::Intel | Vendor::Amd => match_range.start()..(match_range.end() - 1),
            Self::Other => (match_range.start() + 1)..match_range.end(),
        }
    }

    fn format_line(&self, new_line_number: isize) -> String {
        match self {
            Vendor::Nvidia => {
                format!("({new_line_number})")
            }
            Vendor::Intel | Vendor::Amd => {
                format!("{new_line_number}:")
            }
            Vendor::Other => format!(":{new_line_number}"),
        }
    }
}

fn patch_error_message(vendor: Vendor, src: &mut String, line_offset: isize) {
    let re = vendor.regex();
    let mut offset = 0;
    while let Some(result) = re.find_at(src, offset) {
        offset += result.end();
        let range = vendor.line_number_range(result);
        let substr = &src[range];
        if let Ok(line_number) = substr.parse::<isize>() {
            let new_line_number = line_number + line_offset;
            let new_substr = vendor.format_line(new_line_number);
            src.replace_range(result.range(), &new_substr);
        }
    }
}

pub struct GlShader {
    state: Weak<GlGraphicsServer>,
    pub id: glow::Shader,
}

impl ToGlConstant for ShaderKind {
    fn into_gl(self) -> u32 {
        match self {
            ShaderKind::Vertex => glow::VERTEX_SHADER,
            ShaderKind::Fragment => glow::FRAGMENT_SHADER,
        }
    }
}

impl GpuShaderTrait for GlShader {}

impl GlShader {
    pub fn new(
        server: &GlGraphicsServer,
        name: String,
        kind: ShaderKind,
        mut source: String,
        resources: &[ShaderResourceDefinition],
        mut line_offset: isize,
    ) -> Result<Self, FrameworkError> {
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
                            in the {name} GPU program.",
                            resource.name, other_resource.name, resource.binding
                        )));
                    }

                    if resource.name == other_resource.name {
                        return Err(FrameworkError::Custom(format!(
                            "There are two or more resources with same name {} \
                                in the {name} GPU program.",
                            resource.name
                        )));
                    }
                }
            }
        }

        // Generate appropriate texture binding points and uniform blocks for the specified properties.

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
                            ShaderPropertyKind::Float { .. } => {
                                block += &format!("\tfloat {field_name};\n");
                            }
                            ShaderPropertyKind::FloatArray { max_len, .. } => {
                                block += &format!("\tfloat {field_name}[{max_len}];\n");
                            }
                            ShaderPropertyKind::Int { .. } => {
                                block += &format!("\tint {field_name};\n");
                            }
                            ShaderPropertyKind::IntArray { max_len, .. } => {
                                block += &format!("\tint {field_name}[{max_len}];\n");
                            }
                            ShaderPropertyKind::UInt { .. } => {
                                block += &format!("\tuint {field_name};\n");
                            }
                            ShaderPropertyKind::UIntArray { max_len, .. } => {
                                block += &format!("\tuint {field_name}[{max_len}];\n");
                            }
                            ShaderPropertyKind::Bool { .. } => {
                                block += &format!("\tbool {field_name};\n");
                            }
                            ShaderPropertyKind::Vector2 { .. } => {
                                block += &format!("\tvec2 {field_name};\n");
                            }
                            ShaderPropertyKind::Vector2Array { max_len, .. } => {
                                block += &format!("\tvec2 {field_name}[{max_len}];\n");
                            }
                            ShaderPropertyKind::Vector3 { .. } => {
                                block += &format!("\tvec3 {field_name};\n");
                            }
                            ShaderPropertyKind::Vector3Array { max_len, .. } => {
                                block += &format!("\tvec3 {field_name}[{max_len}];\n");
                            }
                            ShaderPropertyKind::Vector4 { .. } => {
                                block += &format!("\tvec4 {field_name};\n");
                            }
                            ShaderPropertyKind::Vector4Array { max_len, .. } => {
                                block += &format!("\tvec4 {field_name}[{max_len}];\n");
                            }
                            ShaderPropertyKind::Matrix2 { .. } => {
                                block += &format!("\tmat2 {field_name};\n");
                            }
                            ShaderPropertyKind::Matrix2Array { max_len, .. } => {
                                block += &format!("\tmat2 {field_name}[{max_len}];\n");
                            }
                            ShaderPropertyKind::Matrix3 { .. } => {
                                block += &format!("\tmat3 {field_name};\n");
                            }
                            ShaderPropertyKind::Matrix3Array { max_len, .. } => {
                                block += &format!("\tmat3 {field_name}[{max_len}];\n");
                            }
                            ShaderPropertyKind::Matrix4 { .. } => {
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
                    source.insert_str(0, &block);
                    line_offset -= count_lines(&block);
                }
            }
        }
        source.insert_str(0, &texture_bindings);
        line_offset -= count_lines(&texture_bindings);

        unsafe {
            let gl_kind = server.gl_kind();
            let initial_lines_count = count_lines(&source);
            let merged_source = prepare_source_code(&source, gl_kind);
            line_offset -= count_lines(&merged_source) - initial_lines_count;

            let shader = server.gl.create_shader(kind.into_gl())?;
            server.gl.shader_source(shader, &merged_source);
            server.gl.compile_shader(shader);

            let status = server.gl.get_shader_compile_status(shader);
            let mut compilation_message = server.gl.get_shader_info_log(shader);

            if !status {
                let vendor_str = server.gl.get_parameter_string(glow::VENDOR).to_lowercase();
                let vendor = Vendor::from_str(vendor_str);
                patch_error_message(vendor, &mut compilation_message, line_offset);
                Log::writeln(
                    MessageKind::Error,
                    format!("Failed to compile {name} shader:\n{compilation_message}"),
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

                Ok(Self {
                    id: shader,
                    state: server.weak(),
                })
            }
        }
    }
}

impl Drop for GlShader {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            unsafe {
                state.gl.delete_shader(self.id);
            }
        }
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
}

impl GlProgram {
    pub fn from_source_and_resources(
        server: &GlGraphicsServer,
        program_name: &str,
        vertex_source: String,
        vertex_source_line_offset: isize,
        fragment_source: String,
        fragment_source_line_offset: isize,
        resources: &[ShaderResourceDefinition],
    ) -> Result<GlProgram, FrameworkError> {
        let program = Self::from_source(
            server,
            program_name,
            vertex_source,
            vertex_source_line_offset,
            fragment_source,
            fragment_source_line_offset,
            resources,
        )?;

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

    fn from_source(
        server: &GlGraphicsServer,
        name: &str,
        vertex_source: String,
        vertex_source_line_offset: isize,
        fragment_source: String,
        fragment_source_line_offset: isize,
        resources: &[ShaderResourceDefinition],
    ) -> Result<GlProgram, FrameworkError> {
        unsafe {
            let vertex_shader = GlShader::new(
                server,
                format!("{name}_VertexShader"),
                ShaderKind::Vertex,
                vertex_source,
                resources,
                vertex_source_line_offset,
            )?;
            let fragment_shader = GlShader::new(
                server,
                format!("{name}_FragmentShader"),
                ShaderKind::Fragment,
                fragment_source,
                resources,
                fragment_source_line_offset,
            )?;
            let program = server.gl.create_program()?;
            server.gl.attach_shader(program, vertex_shader.id);
            server.gl.attach_shader(program, fragment_shader.id);
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
                })
            }
        }
    }
}

impl GpuProgramTrait for GlProgram {}

impl Drop for GlProgram {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            unsafe {
                state.gl.delete_program(self.id);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::gl::program::{patch_error_message, Vendor};

    #[test]
    fn test_line_correction() {
        let mut err_msg = r#"0(62) : error C0000: syntax error, unexpected identifier, expecting '{' at token "vertexPosition"
        0(66) : error C1503: undefined variable "vertexPosition""#.to_string();

        patch_error_message(Vendor::Nvidia, &mut err_msg, 10);

        let expected_result = r#"0(72) : error C0000: syntax error, unexpected identifier, expecting '{' at token "vertexPosition"
        0(76) : error C1503: undefined variable "vertexPosition""#;

        assert_eq!(err_msg, expected_result);
    }
}
