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

use crate::device::VkDevice;
use crate::ToVkType;
// DeviceV1_0 is no longer needed in ash 0.38
use ash::vk;
use fyrox_graphics::{
    error::FrameworkError,
    gpu_program::{
        GpuProgramTrait, GpuShaderTrait, ShaderKind, ShaderResourceDefinition, ShaderResourceKind,
    },
};
use std::collections::HashMap;
use std::ffi::CString;
use std::rc::Rc;
use std::sync::Arc;

impl ToVkType<vk::ShaderStageFlags> for &ShaderKind {
    fn to_vk(self) -> vk::ShaderStageFlags {
        match self {
            ShaderKind::Vertex => vk::ShaderStageFlags::VERTEX,
            ShaderKind::Fragment => vk::ShaderStageFlags::FRAGMENT,
        }
    }
}

/// Shader compiler using shaderc.
pub struct ShaderCompiler {
    /// The shaderc compiler instance.
    compiler: shaderc::Compiler,
}

impl ShaderCompiler {
    /// Creates a new shader compiler.
    pub fn new() -> Result<Self, FrameworkError> {
        // shaderc::Compiler::new returns Option; construct error explicitly if None
        let compiler = shaderc::Compiler::new().ok_or(FrameworkError::Custom(
            "Failed to create shader compiler".to_string(),
        ))?;

        Ok(Self { compiler })
    }

    /// Compiles GLSL source code to SPIR-V.
    pub fn compile_glsl_to_spirv(
        &mut self,
        source: &str,
        shader_kind: &ShaderKind,
        input_file_name: &str,
        entry_point_name: &str,
        additional_options: Option<&shaderc::CompileOptions>,
    ) -> Result<Vec<u32>, FrameworkError> {
        let shaderc_kind = match shader_kind {
            ShaderKind::Vertex => shaderc::ShaderKind::Vertex,
            ShaderKind::Fragment => shaderc::ShaderKind::Fragment,
        };

        let result = self
            .compiler
            .compile_into_spirv(
                source,
                shaderc_kind,
                input_file_name,
                entry_point_name,
                additional_options,
            )
            .map_err(|e| FrameworkError::Custom(format!("Shader compilation failed: {}", e)))?;

        if result.get_num_warnings() > 0 {
            log::warn!(
                "Shader compilation warnings: {}",
                result.get_warning_messages()
            );
        }

        Ok(result.as_binary().to_vec())
    }
}

/// Vulkan shader implementation.
pub struct VkGpuShader {
    /// The shader module.
    module: vk::ShaderModule,
    /// Shader kind.
    kind: ShaderKind,
    /// Entry point name.
    entry_point: CString,
    /// Device reference.
    device: Arc<VkDevice>,
    /// SPIR-V bytecode.
    spirv: Vec<u32>,
}

impl VkGpuShader {
    /// Creates a stub shader (minimal valid shader for testing).
    pub fn new_stub(device: Arc<VkDevice>, kind: ShaderKind) -> Result<Self, FrameworkError> {
        // Minimal SPIR-V stub shaders
        let spirv = match kind {
            ShaderKind::Vertex => {
                // Minimal vertex shader SPIR-V
                vec![
                    0x07230203, 0x00010000, 0x00080001, 0x00000006, 0x00000000, 0x00020011,
                    0x00000001, 0x0006000b, 0x00000001, 0x4c534c47, 0x6474732e, 0x3035342e,
                    0x00000000,
                ]
            }
            ShaderKind::Fragment => {
                // Minimal fragment shader SPIR-V
                vec![
                    0x07230203, 0x00010000, 0x00080001, 0x00000008, 0x00000000, 0x00020011,
                    0x00000001, 0x0006000b, 0x00000001, 0x4c534c47, 0x6474732e, 0x3035342e,
                    0x00000000,
                ]
            }
        };

        let create_info = vk::ShaderModuleCreateInfo::builder().code(&spirv).build();

        let module = unsafe {
            device
                .device
                .create_shader_module(&create_info, None)
                .map_err(|e| {
                    FrameworkError::Custom(format!("Failed to create shader module: {:?}", e))
                })?
        };

        let entry_point = CString::new("main")
            .map_err(|e| FrameworkError::Custom(format!("Invalid entry point name: {}", e)))?;

        Ok(Self {
            module,
            kind,
            entry_point,
            device,
            spirv,
        })
    }

    /// Creates a new Vulkan shader.
    pub fn new(
        device: Arc<VkDevice>,
        name: String,
        kind: ShaderKind,
        source: String,
        resources: &[ShaderResourceDefinition],
        _line_offset: isize,
    ) -> Result<Self, FrameworkError> {
        let mut compiler = ShaderCompiler::new()?;

        // Add resource bindings to the source
        let processed_source = Self::process_source_with_resources(source, resources)?;

        let spirv =
            compiler.compile_glsl_to_spirv(&processed_source, &kind, &name, "main", None)?;

        let create_info = vk::ShaderModuleCreateInfo::builder().code(&spirv).build();

        let module = unsafe {
            device
                .device
                .create_shader_module(&create_info, None)
                .map_err(|e| {
                    FrameworkError::Custom(format!("Failed to create shader module: {:?}", e))
                })?
        };

        let entry_point = CString::new("main")
            .map_err(|e| FrameworkError::Custom(format!("Invalid entry point name: {}", e)))?;

        Ok(Self {
            module,
            kind,
            entry_point,
            device,
            spirv,
        })
    }

    /// Processes shader source code to add resource bindings.
    fn process_source_with_resources(
        mut source: String,
        resources: &[ShaderResourceDefinition],
    ) -> Result<String, FrameworkError> {
        // Add version directive if not present
        if !source.contains("#version") {
            source = format!("#version 450 core\n{}", source);
        }

        // Find position after version directive to insert bindings and shared code
        let insert_pos = if let Some(version_end) = source.find('\n') {
            version_end + 1
        } else {
            0
        };

        let mut bindings_code = String::new();

        // Add resource bindings with proper Vulkan descriptor set layout
        let mut binding_index = 0;
        for resource in resources {
            let binding_code = match &resource.kind {
                ShaderResourceKind::Texture { kind, .. } => {
                    let sampler_type = match kind {
                        fyrox_graphics::gpu_program::SamplerKind::Sampler1D => "sampler1D",
                        fyrox_graphics::gpu_program::SamplerKind::Sampler2D => "sampler2D",
                        fyrox_graphics::gpu_program::SamplerKind::Sampler3D => "sampler3D",
                        fyrox_graphics::gpu_program::SamplerKind::SamplerCube => "samplerCube",
                        fyrox_graphics::gpu_program::SamplerKind::USampler1D => "usampler1D",
                        fyrox_graphics::gpu_program::SamplerKind::USampler2D => "usampler2D",
                        fyrox_graphics::gpu_program::SamplerKind::USampler3D => "usampler3D",
                        fyrox_graphics::gpu_program::SamplerKind::USamplerCube => "usamplerCube",
                    };
                    format!(
                        "layout(set = 0, binding = {}) uniform {} {};\n",
                        binding_index, sampler_type, resource.name
                    )
                }
                ShaderResourceKind::PropertyGroup(properties) => {
                    // Build uniform block content
                    let mut block_content = String::new();
                    for prop in properties {
                        let type_str = match &prop.kind {
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Float { .. } => {
                                "float"
                            }
                            fyrox_graphics::gpu_program::ShaderPropertyKind::FloatArray {
                                ..
                            } => "float",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Int { .. } => "int",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::IntArray {
                                ..
                            } => "int",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::UInt { .. } => "uint",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::UIntArray {
                                ..
                            } => "uint",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Vector2 { .. } => {
                                "vec2"
                            }
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Vector2Array {
                                ..
                            } => "vec2",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Vector3 { .. } => {
                                "vec3"
                            }
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Vector3Array {
                                ..
                            } => "vec3",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Vector4 { .. } => {
                                "vec4"
                            }
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Vector4Array {
                                ..
                            } => "vec4",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Matrix2 { .. } => {
                                "mat2"
                            }
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Matrix2Array {
                                ..
                            } => "mat2",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Matrix3 { .. } => {
                                "mat3"
                            }
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Matrix3Array {
                                ..
                            } => "mat3",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Matrix4 { .. } => {
                                "mat4"
                            }
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Matrix4Array {
                                ..
                            } => "mat4",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Bool { .. } => "bool",
                            fyrox_graphics::gpu_program::ShaderPropertyKind::Color { .. } => "vec4",
                        };

                        // Handle array types
                        let array_suffix = match &prop.kind {
                            fyrox_graphics::gpu_program::ShaderPropertyKind::FloatArray {
                                max_len,
                                ..
                            }
                            | fyrox_graphics::gpu_program::ShaderPropertyKind::IntArray {
                                max_len,
                                ..
                            }
                            | fyrox_graphics::gpu_program::ShaderPropertyKind::UIntArray {
                                max_len,
                                ..
                            }
                            | fyrox_graphics::gpu_program::ShaderPropertyKind::Vector2Array {
                                max_len,
                                ..
                            }
                            | fyrox_graphics::gpu_program::ShaderPropertyKind::Vector3Array {
                                max_len,
                                ..
                            }
                            | fyrox_graphics::gpu_program::ShaderPropertyKind::Vector4Array {
                                max_len,
                                ..
                            }
                            | fyrox_graphics::gpu_program::ShaderPropertyKind::Matrix2Array {
                                max_len,
                                ..
                            }
                            | fyrox_graphics::gpu_program::ShaderPropertyKind::Matrix3Array {
                                max_len,
                                ..
                            }
                            | fyrox_graphics::gpu_program::ShaderPropertyKind::Matrix4Array {
                                max_len,
                                ..
                            } => {
                                format!("[{}]", max_len)
                            }
                            _ => String::new(),
                        };

                        block_content.push_str(&format!(
                            "    {} {}{};\n",
                            type_str, prop.name, array_suffix
                        ));
                    }

                    format!(
                        "layout(set = 0, binding = {}) uniform U{} {{\n{}}} {};\n",
                        binding_index, resource.name, block_content, resource.name
                    )
                }
            };

            bindings_code.push_str(&binding_code);
            binding_index += 1;
        }

        // Include shared GLSL library (shared.glsl content)
        let shared_glsl = include_str!("../../fyrox-graphics-gl/src/shaders/shared.glsl");
        let full_preamble = format!(
            "// Vulkan/SPIR-V compatible shader\n{}\n// Shared library functions\n{}\n",
            bindings_code, shared_glsl
        );

        // Insert the preamble after version directive
        source.insert_str(insert_pos, &full_preamble);

        Ok(source)
    }

    /// Gets the Vulkan shader module.
    pub fn vk_module(&self) -> vk::ShaderModule {
        self.module
    }

    /// Gets the shader stage flags.
    pub fn stage_flags(&self) -> vk::ShaderStageFlags {
        (&self.kind).to_vk()
    }

    /// Gets the entry point name.
    pub fn entry_point(&self) -> &CString {
        &self.entry_point
    }
}

impl GpuShaderTrait for VkGpuShader {
    // Empty trait implementation
}

impl Drop for VkGpuShader {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_shader_module(self.module, None);
        }
    }
}

/// Vulkan GPU program implementation.
pub struct VkGpuProgram {
    /// Vertex shader.
    vertex_shader: Rc<VkGpuShader>,
    /// Fragment shader.
    fragment_shader: Rc<VkGpuShader>,
    /// Pipeline layout.
    pipeline_layout: vk::PipelineLayout,
    /// Descriptor set layout.
    descriptor_set_layout: vk::DescriptorSetLayout,
    /// Uniform locations map.
    #[allow(dead_code)]
    uniform_locations: HashMap<String, usize>,
    /// Device reference.
    device: Arc<VkDevice>,
}

impl VkGpuProgram {
    /// Creates a stub Vulkan GPU program (for development/testing).
    pub fn new_stub(device: Arc<VkDevice>, _name: &str) -> Result<Self, FrameworkError> {
        // Create minimal stub shaders
        let vertex_shader = Rc::new(VkGpuShader::new_stub(device.clone(), ShaderKind::Vertex)?);
        let fragment_shader = Rc::new(VkGpuShader::new_stub(device.clone(), ShaderKind::Fragment)?);

        // Create empty descriptor set layout
        let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&[])
            .build();

        let descriptor_set_layout = unsafe {
            device
                .device
                .create_descriptor_set_layout(&descriptor_set_layout_info, None)
                .map_err(|e| {
                    FrameworkError::Custom(format!(
                        "Failed to create descriptor set layout: {:?}",
                        e
                    ))
                })?
        };

        // Create empty pipeline layout
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&[descriptor_set_layout])
            .push_constant_ranges(&[])
            .build();

        let pipeline_layout = unsafe {
            device
                .device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| {
                    FrameworkError::Custom(format!("Failed to create pipeline layout: {:?}", e))
                })?
        };

        Ok(Self {
            vertex_shader,
            fragment_shader,
            device,
            descriptor_set_layout,
            pipeline_layout,
            uniform_locations: HashMap::new(),
        })
    }

    /// Creates a new Vulkan GPU program from shaders.
    pub fn new_from_shaders(
        device: Arc<VkDevice>,
        _name: &str,
        vertex_shader: &VkGpuShader,
        fragment_shader: &VkGpuShader,
        resources: &[ShaderResourceDefinition],
    ) -> Result<Self, FrameworkError> {
        // Create descriptor set layout
        let mut bindings = Vec::new();
        let mut uniform_locations = HashMap::new();

        for (index, resource) in resources.iter().enumerate() {
            let binding = match &resource.kind {
                ShaderResourceKind::Texture { .. } => {
                    uniform_locations.insert(resource.name.to_string(), index);
                    vk::DescriptorSetLayoutBinding::builder()
                        .binding(index as u32)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                        .build()
                }
                ShaderResourceKind::PropertyGroup(_) => {
                    uniform_locations.insert(resource.name.to_string(), index);
                    vk::DescriptorSetLayoutBinding::builder()
                        .binding(index as u32)
                        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                        .descriptor_count(1)
                        .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
                        .build()
                }
            };
            bindings.push(binding);
        }

        let descriptor_set_layout_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .build();

        let descriptor_set_layout = unsafe {
            device
                .device
                .create_descriptor_set_layout(&descriptor_set_layout_info, None)
                .map_err(|e| {
                    FrameworkError::Custom(format!(
                        "Failed to create descriptor set layout: {:?}",
                        e
                    ))
                })?
        };

        // Create pipeline layout
        let set_layouts = [descriptor_set_layout];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&[])
            .build();

        let pipeline_layout = unsafe {
            device
                .device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| {
                    FrameworkError::Custom(format!("Failed to create pipeline layout: {:?}", e))
                })?
        };

        // Copy ShaderKind (cannot clone or copy directly)
        let vertex_kind = match vertex_shader.kind {
            ShaderKind::Vertex => ShaderKind::Vertex,
            ShaderKind::Fragment => ShaderKind::Fragment,
        };
        let fragment_kind = match fragment_shader.kind {
            ShaderKind::Vertex => ShaderKind::Vertex,
            ShaderKind::Fragment => ShaderKind::Fragment,
        };

        Ok(Self {
            vertex_shader: Rc::new(VkGpuShader {
                module: vertex_shader.module,
                kind: vertex_kind,
                entry_point: vertex_shader.entry_point.clone(),
                device: device.clone(),
                spirv: vertex_shader.spirv.clone(),
            }),
            fragment_shader: Rc::new(VkGpuShader {
                module: fragment_shader.module,
                kind: fragment_kind,
                entry_point: fragment_shader.entry_point.clone(),
                device: device.clone(),
                spirv: fragment_shader.spirv.clone(),
            }),
            pipeline_layout,
            descriptor_set_layout,
            uniform_locations,
            device,
        })
    }

    /// Gets the pipeline layout.
    pub fn pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }

    /// Gets the descriptor set layout.
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }

    /// Gets the vertex shader.
    pub fn vertex_shader(&self) -> &VkGpuShader {
        &self.vertex_shader
    }

    /// Gets the fragment shader.
    pub fn fragment_shader(&self) -> &VkGpuShader {
        &self.fragment_shader
    }
}

impl GpuProgramTrait for VkGpuProgram {
    // Empty trait implementation
}

impl Drop for VkGpuProgram {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device
                .device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

/// Creates a Vulkan GPU shader.
pub fn create_shader(
    device: Arc<VkDevice>,
    name: String,
    kind: ShaderKind,
    source: String,
    resources: &[ShaderResourceDefinition],
    line_offset: isize,
) -> Result<Rc<dyn GpuShaderTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuShader::new(
        device,
        name,
        kind,
        source,
        resources,
        line_offset,
    )?))
}

/// Creates a Vulkan GPU program from shaders.
pub fn create_program_from_shaders(
    device: Arc<VkDevice>,
    name: &str,
    vertex_shader: &dyn GpuShaderTrait,
    fragment_shader: &dyn GpuShaderTrait,
    resources: &[ShaderResourceDefinition],
) -> Result<Rc<dyn GpuProgramTrait>, FrameworkError> {
    let vertex_shader = vertex_shader
        .as_any()
        .downcast_ref::<VkGpuShader>()
        .ok_or_else(|| FrameworkError::Custom("Invalid vertex shader type".to_string()))?;

    let fragment_shader = fragment_shader
        .as_any()
        .downcast_ref::<VkGpuShader>()
        .ok_or_else(|| FrameworkError::Custom("Invalid fragment shader type".to_string()))?;

    Ok(Rc::new(VkGpuProgram::new_from_shaders(
        device,
        name,
        vertex_shader,
        fragment_shader,
        resources,
    )?))
}
