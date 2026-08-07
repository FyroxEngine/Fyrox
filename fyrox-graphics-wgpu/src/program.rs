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

use crate::format_helpers::{
    sample_type_for_format, SAMPLER_BINDING_OFFSET, UNIFORM_BINDING_OFFSET,
};
use crate::server::WgpuGraphicsServer;
use fyrox_graphics::{
    core::log::{Log, MessageKind},
    error::FrameworkError,
    gpu_program::{
        GpuProgramTrait, GpuShaderTrait, SamplerKind, ShaderKind, ShaderPropertyKind,
        ShaderResourceDefinition, ShaderResourceKind,
    },
};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Weak;

/// Returns the WGSL texture type for a given sampler kind.
fn wgsl_texture_type(kind: SamplerKind) -> &'static str {
    match kind {
        SamplerKind::Sampler1D => "texture_1d<f32>",
        SamplerKind::Sampler2D => "texture_2d<f32>",
        SamplerKind::Sampler3D => "texture_3d<f32>",
        SamplerKind::SamplerCube => "texture_cube<f32>",
        SamplerKind::USampler1D => "texture_1d<u32>",
        SamplerKind::USampler2D => "texture_2d<u32>",
        SamplerKind::USampler3D => "texture_3d<u32>",
        SamplerKind::USamplerCube => "texture_cube<u32>",
        SamplerKind::DepthSampler2D => "texture_depth_2d",
        SamplerKind::DepthSamplerCube => "texture_depth_cube",
    }
}

/// Returns the WGSL type string for a shader property kind.
fn wgsl_property_type(kind: &ShaderPropertyKind) -> String {
    match kind {
        ShaderPropertyKind::Float { .. } => "f32".into(),
        ShaderPropertyKind::FloatArray { max_len, .. } => format!("array<vec4f, {max_len}>"),
        ShaderPropertyKind::Int { .. } => "i32".into(),
        ShaderPropertyKind::IntArray { max_len, .. } => format!("array<vec4i, {max_len}>"),
        ShaderPropertyKind::UInt { .. } => "u32".into(),
        ShaderPropertyKind::UIntArray { max_len, .. } => format!("array<vec4u, {max_len}>"),
        ShaderPropertyKind::Bool { .. } => "u32".into(),
        ShaderPropertyKind::Vector2 { .. } => "vec2f".into(),
        ShaderPropertyKind::Vector2Array { max_len, .. } => format!("array<vec4f, {max_len}>"),
        ShaderPropertyKind::Vector3 { .. } => "vec3f".into(),
        ShaderPropertyKind::Vector3Array { max_len, .. } => format!("array<vec3f, {max_len}>"),
        ShaderPropertyKind::Vector4 { .. } => "vec4f".into(),
        ShaderPropertyKind::Vector4Array { max_len, .. } => format!("array<vec4f, {max_len}>"),
        ShaderPropertyKind::Matrix2 { .. } => "mat2x2f".into(),
        ShaderPropertyKind::Matrix2Array { max_len, .. } => format!("array<mat2x2f, {max_len}>"),
        ShaderPropertyKind::Matrix3 { .. } => "mat3x3f".into(),
        ShaderPropertyKind::Matrix3Array { max_len, .. } => format!("array<mat3x3f, {max_len}>"),
        ShaderPropertyKind::Matrix4 { .. } => "mat4x4f".into(),
        ShaderPropertyKind::Matrix4Array { max_len, .. } => format!("array<mat4x4f, {max_len}>"),
        ShaderPropertyKind::Color { .. } => "vec4f".into(),
    }
}

/// Generates WGSL `@group(0) @binding(N)` declarations for textures, samplers,
/// and uniform buffers from the shader resource definitions.
fn generate_wgsl_declarations(resources: &[ShaderResourceDefinition]) -> String {
    let mut decls = String::new();

    for res in resources {
        match res.kind {
            ShaderResourceKind::Texture { kind, .. } => {
                let tex_type = wgsl_texture_type(kind);
                decls += &format!(
                    "@group(0) @binding({}) var {}_tex: {};\n",
                    res.binding, res.name, tex_type
                );
                decls += &format!(
                    "@group(0) @binding({}) var {}_samp: sampler;\n",
                    res.binding + SAMPLER_BINDING_OFFSET,
                    res.name
                );
            }
            ShaderResourceKind::PropertyGroup(ref fields) => {
                if fields.is_empty() {
                    continue;
                }
                decls += &format!("struct T{} {{\n", res.name);
                for f in fields {
                    let n = &f.name;
                    let ty = wgsl_property_type(&f.kind);
                    decls += &format!("    {n}: {ty},\n");
                }
                decls += "}\n";
                decls += &format!(
                    "@group(0) @binding({}) var<uniform> {}: T{};\n",
                    res.binding + UNIFORM_BINDING_OFFSET,
                    res.name,
                    res.name
                );
            }
        }
    }

    decls
}

/// Compiles WGSL source into a wgpu shader module.
fn compile_wgsl(
    device: &wgpu::Device,
    name: &str,
    wgsl: &str,
) -> Result<wgpu::ShaderModule, FrameworkError> {
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(wgsl)),
    });
    Log::writeln(
        MessageKind::Information,
        format!("Shader {name} compiled successfully!"),
    );
    Ok(shader_module)
}

/// Creates bind group layout using actual texture formats when available.
/// `texture_formats` provides the actual wgpu format for specific bindings.
fn create_bind_group_layout_with_formats(
    device: &wgpu::Device,
    resources: &[ShaderResourceDefinition],
    texture_formats: &[(usize, wgpu::TextureFormat)],
) -> wgpu::BindGroupLayout {
    let fmt_map: std::collections::HashMap<usize, wgpu::TextureFormat> =
        texture_formats.iter().copied().collect();
    let mut entries = Vec::new();
    for res in resources {
        match res.kind {
            ShaderResourceKind::Texture { kind, .. } => {
                let vd = match kind {
                    SamplerKind::Sampler1D | SamplerKind::USampler1D => {
                        wgpu::TextureViewDimension::D1
                    }
                    SamplerKind::Sampler2D
                    | SamplerKind::USampler2D
                    | SamplerKind::DepthSampler2D => wgpu::TextureViewDimension::D2,
                    SamplerKind::Sampler3D | SamplerKind::USampler3D => {
                        wgpu::TextureViewDimension::D3
                    }
                    SamplerKind::SamplerCube
                    | SamplerKind::USamplerCube
                    | SamplerKind::DepthSamplerCube => wgpu::TextureViewDimension::Cube,
                };
                // Use actual texture format if available, otherwise fall back to kind-based inference
                let st = if let Some(&fmt) = fmt_map.get(&res.binding) {
                    sample_type_for_format(fmt)
                } else {
                    match kind {
                        SamplerKind::DepthSampler2D | SamplerKind::DepthSamplerCube => {
                            wgpu::TextureSampleType::Depth
                        }
                        SamplerKind::USampler1D
                        | SamplerKind::USampler2D
                        | SamplerKind::USampler3D
                        | SamplerKind::USamplerCube => wgpu::TextureSampleType::Uint,
                        _ => wgpu::TextureSampleType::Float { filterable: true },
                    }
                };
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding: res.binding as u32,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: st,
                        view_dimension: vd,
                        multisampled: false,
                    },
                    count: None,
                });
                let sampler_binding = if matches!(
                    st,
                    wgpu::TextureSampleType::Float { filterable: false }
                        | wgpu::TextureSampleType::Uint
                        | wgpu::TextureSampleType::Sint
                ) {
                    wgpu::SamplerBindingType::NonFiltering
                } else {
                    wgpu::SamplerBindingType::Filtering
                };
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding: (res.binding + SAMPLER_BINDING_OFFSET) as u32,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(sampler_binding),
                    count: None,
                });
            }
            ShaderResourceKind::PropertyGroup { .. } => {
                entries.push(wgpu::BindGroupLayoutEntry {
                    binding: (res.binding + UNIFORM_BINDING_OFFSET) as u32,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                });
            }
        }
    }
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ShaderBindGroupLayout"),
        entries: &entries,
    })
}

/// Wgpu implementation of [`GpuShaderTrait`](fyrox_graphics::gpu_program::GpuShaderTrait).
///
/// Wraps a compiled [`wgpu::ShaderModule`]. Shaders are compiled from WGSL source
/// with automatically generated resource binding declarations prepended.
pub struct WgpuShader {
    _server: Weak<WgpuGraphicsServer>,
    module: wgpu::ShaderModule,
    _kind: ShaderKind,
}

impl GpuShaderTrait for WgpuShader {}

impl WgpuShader {
    /// Compiles a WGSL shader from source with resource binding declarations.
    ///
    /// The compilation pipeline:
    /// 1. Validates resource definitions for duplicate bindings/names
    /// 2. Generates `@group(0) @binding(N)` WGSL declarations via [`generate_wgsl_declarations`]
    /// 3. Prepends declarations + [`shared.wgsl`](shaders/shared.wgsl) to the user source
    /// 4. Compiles the combined WGSL as a [`wgpu::ShaderModule`]
    pub fn new(
        server: &WgpuGraphicsServer,
        name: String,
        kind: ShaderKind,
        source: String,
        resources: &[ShaderResourceDefinition],
        _line_offset: isize,
    ) -> Result<Self, FrameworkError> {
        for r in resources {
            for o in resources {
                if std::ptr::eq(r, o) {
                    continue;
                }
                if std::mem::discriminant(&r.kind) == std::mem::discriminant(&o.kind) {
                    if r.binding == o.binding {
                        return Err(FrameworkError::Custom(format!(
                            "Resource {} and {} same binding {}",
                            r.name, o.name, r.binding
                        )));
                    }
                    if r.name == o.name {
                        return Err(FrameworkError::Custom(format!(
                            "Duplicate resource name {}",
                            r.name
                        )));
                    }
                }
            }
        }

        let declarations = generate_wgsl_declarations(resources);
        let shared = include_str!("shaders/shared.wgsl");

        let mut wgsl = String::new();
        wgsl += &declarations;
        wgsl += shared;
        wgsl += "\n";
        wgsl += &source;

        let module = compile_wgsl(&server.state.device, &name, &wgsl)?;
        Ok(Self {
            _server: server.weak_ref(),
            module,
            _kind: kind,
        })
    }

    /// Returns a reference to the compiled [`wgpu::ShaderModule`].
    pub fn wgpu_module(&self) -> &wgpu::ShaderModule {
        &self.module
    }
}

/// Wgpu implementation of [`GpuProgramTrait`](fyrox_graphics::gpu_program::GpuProgramTrait).
///
/// A shader program consisting of a vertex and fragment [`wgpu::ShaderModule`],
/// along with resource definitions and lazily-cached bind group / pipeline layouts.
///
/// The layout cache is keyed on the actual texture formats passed to
/// [`get_or_create_layouts`](Self::get_or_create_layouts), ensuring correct
/// sample type inference for each unique set of bound textures.
pub struct WgpuProgram {
    server: Weak<WgpuGraphicsServer>,
    name: String,
    vertex_module: wgpu::ShaderModule,
    fragment_module: wgpu::ShaderModule,
    resources: Vec<ShaderResourceDefinition>,
    cached_layouts: RefCell<
        HashMap<
            Vec<(usize, wgpu::TextureSampleType)>,
            (wgpu::BindGroupLayout, wgpu::PipelineLayout),
        >,
    >,
}

impl GpuProgramTrait for WgpuProgram {}

impl WgpuProgram {
    /// Creates a program by compiling vertex and fragment shaders from source.
    ///
    /// Both shaders share the same resource definitions. The vertex shader is
    /// named `{name}_VS` and the fragment shader `{name}_FS`.
    pub fn from_source(
        server: &WgpuGraphicsServer,
        name: &str,
        vs: String,
        vs_off: isize,
        fs: String,
        fs_off: isize,
        resources: &[ShaderResourceDefinition],
    ) -> Result<Self, FrameworkError> {
        let vert = WgpuShader::new(
            server,
            format!("{name}_VS"),
            ShaderKind::Vertex,
            vs,
            resources,
            vs_off,
        )?;
        let frag = WgpuShader::new(
            server,
            format!("{name}_FS"),
            ShaderKind::Fragment,
            fs,
            resources,
            fs_off,
        )?;
        Self::from_modules(server, name, &vert, &frag, resources)
    }

    /// Creates a program from pre-compiled shaders.
    ///
    /// The shaders must be [`WgpuShader`] instances (downcast from trait objects).
    pub fn from_shaders(
        server: &WgpuGraphicsServer,
        name: &str,
        vs: &fyrox_graphics::gpu_program::GpuShader,
        fs: &fyrox_graphics::gpu_program::GpuShader,
        resources: &[ShaderResourceDefinition],
    ) -> Result<Self, FrameworkError> {
        let vert = vs
            .as_any()
            .downcast_ref::<WgpuShader>()
            .ok_or_else(|| FrameworkError::Custom("Expected WgpuShader".into()))?;
        let frag = fs
            .as_any()
            .downcast_ref::<WgpuShader>()
            .ok_or_else(|| FrameworkError::Custom("Expected WgpuShader".into()))?;
        Self::from_modules(server, name, vert, frag, resources)
    }

    fn from_modules(
        server: &WgpuGraphicsServer,
        name: &str,
        vert: &WgpuShader,
        frag: &WgpuShader,
        resources: &[ShaderResourceDefinition],
    ) -> Result<Self, FrameworkError> {
        Ok(Self {
            server: server.weak_ref(),
            name: name.to_owned(),
            vertex_module: vert.module.clone(),
            fragment_module: frag.module.clone(),
            resources: resources.to_vec(),
            cached_layouts: RefCell::new(HashMap::new()),
        })
    }

    /// Lazily create bind group layout + pipeline layout based on actual texture formats.
    /// `texture_formats` maps resource binding -> actual wgpu texture format for textures.
    pub fn get_or_create_layouts(
        &self,
        texture_formats: &[(usize, wgpu::TextureFormat)],
    ) -> (wgpu::BindGroupLayout, wgpu::PipelineLayout) {
        let sample_types: Vec<_> = texture_formats
            .iter()
            .map(|(loc, fmt)| (*loc, sample_type_for_format(*fmt)))
            .collect();

        let mut cache = self.cached_layouts.borrow_mut();

        if let Some((bgl, pl)) = cache.get(&sample_types) {
            return (bgl.clone(), pl.clone());
        }

        let server = self
            .server
            .upgrade()
            .expect("WgpuGraphicsServer dropped before WgpuProgram");

        let bgl = create_bind_group_layout_with_formats(
            &server.state.device,
            &self.resources,
            texture_formats,
        );

        let pl = server
            .state
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("{}_PL", self.name)),
                bind_group_layouts: &[Some(&bgl)],
                ..Default::default()
            });

        let result = (bgl.clone(), pl.clone());

        cache.insert(sample_types, result.clone());

        result
    }

    /// Returns a reference to the vertex shader module.
    pub fn vertex_module(&self) -> &wgpu::ShaderModule {
        &self.vertex_module
    }
    /// Returns a reference to the fragment shader module.
    pub fn fragment_module(&self) -> &wgpu::ShaderModule {
        &self.fragment_module
    }
    /// Returns the resource definitions for this program.
    pub fn resources(&self) -> &[ShaderResourceDefinition] {
        &self.resources
    }
    /// Returns the program name (used for debug labels).
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_wgsl_declarations_textures() {
        let resources = vec![
            ShaderResourceDefinition {
                name: "diffuseTexture".into(),
                kind: ShaderResourceKind::Texture {
                    kind: SamplerKind::Sampler2D,
                    fallback: Default::default(),
                },
                binding: 0,
            },
            ShaderResourceDefinition {
                name: "shadowMap".into(),
                kind: ShaderResourceKind::Texture {
                    kind: SamplerKind::SamplerCube,
                    fallback: Default::default(),
                },
                binding: 1,
            },
        ];
        let decls = generate_wgsl_declarations(&resources);
        assert!(decls.contains("@group(0) @binding(0) var diffuseTexture_tex: texture_2d<f32>;"));
        assert!(decls.contains("@group(0) @binding(100) var diffuseTexture_samp: sampler;"));
        assert!(decls.contains("@group(0) @binding(1) var shadowMap_tex: texture_cube<f32>;"));
        assert!(decls.contains("@group(0) @binding(101) var shadowMap_samp: sampler;"));
    }

    #[test]
    fn test_generate_wgsl_declarations_uniforms() {
        use fyrox_graphics::gpu_program::ShaderProperty;
        let resources = vec![ShaderResourceDefinition {
            name: "properties".into(),
            kind: ShaderResourceKind::PropertyGroup(vec![
                ShaderProperty::new("field0", ShaderPropertyKind::Float { value: 0.0 }),
                ShaderProperty::new(
                    "field1",
                    ShaderPropertyKind::Vector3 {
                        value: Default::default(),
                    },
                ),
                ShaderProperty::new("field2", ShaderPropertyKind::Bool { value: false }),
            ]),
            binding: 0,
        }];
        let decls = generate_wgsl_declarations(&resources);
        assert!(decls.contains("struct Tproperties {"));
        assert!(decls.contains("field0: f32,"));
        assert!(decls.contains("field1: vec3f,"));
        assert!(decls.contains("field2: u32,")); // Bool → u32
        assert!(decls.contains("@group(0) @binding(200) var<uniform> properties: Tproperties;"));
    }

    #[test]
    fn test_wgsl_texture_type_mapping() {
        assert_eq!(wgsl_texture_type(SamplerKind::Sampler2D), "texture_2d<f32>");
        assert_eq!(wgsl_texture_type(SamplerKind::Sampler3D), "texture_3d<f32>");
        assert_eq!(
            wgsl_texture_type(SamplerKind::SamplerCube),
            "texture_cube<f32>"
        );
        assert_eq!(
            wgsl_texture_type(SamplerKind::USampler2D),
            "texture_2d<u32>"
        );
    }

    #[test]
    fn test_wgsl_property_type_mapping() {
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Float { value: 0.0 }),
            "f32"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Int { value: 0 }),
            "i32"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::UInt { value: 0 }),
            "u32"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Bool { value: false }),
            "u32"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Vector2 {
                value: Default::default()
            }),
            "vec2f"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Vector3 {
                value: Default::default()
            }),
            "vec3f"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Vector4 {
                value: Default::default()
            }),
            "vec4f"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Matrix2 {
                value: Default::default()
            }),
            "mat2x2f"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Matrix3 {
                value: Default::default()
            }),
            "mat3x3f"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Matrix4 {
                value: Default::default()
            }),
            "mat4x4f"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255
            }),
            "vec4f"
        );
        assert_eq!(
            wgsl_property_type(&ShaderPropertyKind::FloatArray {
                max_len: 32,
                value: vec![]
            }),
            "array<vec4f, 32>"
        );
    }
}
