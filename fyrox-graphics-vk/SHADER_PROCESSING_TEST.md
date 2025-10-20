# Test: Shader Processing Verification

This file demonstrates the actual output of the shader preprocessing system.

## Input: Simple Blit Shader

```glsl
layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec2 vertexTexCoord;

out vec2 texCoord;

void main()
{
    texCoord = vertexTexCoord;
    gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
}
```

**Resources:**
- Texture: `diffuseTexture` (Sampler2D, binding 0)
- PropertyGroup: `properties` with `worldViewProjection: Matrix4` (binding 1)

## Output: Processed for Vulkan

```glsl
#version 450 core
// Vulkan/SPIR-V compatible shader

// Resource bindings
layout(set = 0, binding = 0) uniform sampler2D diffuseTexture;
layout(set = 0, binding = 1) uniform Uproperties {
    mat4 worldViewProjection;
} properties;

// Shared library functions (excerpt)
const float PI = 3.14159;

bool S_SolveQuadraticEq(float a, float b, float c, out float minT, out float maxT) {
    float twoA = 2.0 * a;
    float det = b * b - 2.0 * twoA * c;
    if (det < 0.0) {
        minT = 0.0;
        maxT = 0.0;
        return false;
    }
    float sqrtDet = sqrt(det);
    float root1 = (-b - sqrtDet) / twoA;
    float root2 = (-b + sqrtDet) / twoA;
    minT = min(root1, root2);
    maxT = max(root1, root2);
    return true;
}

vec3 S_UnProject(vec3 screenPos, mat4 matrix) {
    vec4 clipSpacePos = vec4(screenPos * 2.0 - 1.0, 1.0);
    vec4 position = matrix * clipSpacePos;
    return position.xyz / position.w;
}

vec4 S_SRGBToLinear(vec4 color) {
    vec3 a = color.rgb / 12.92;
    vec3 b = pow((color.rgb + 0.055) / 1.055, vec3(2.4));
    vec3 c = step(vec3(0.04045), color.rgb);
    vec3 rgb = mix(a, b, c);
    return vec4(rgb, color.a);
}

// ... (381 total lines of shared functions) ...

// Original shader code
layout (location = 0) in vec3 vertexPosition;
layout (location = 1) in vec2 vertexTexCoord;

out vec2 texCoord;

void main()
{
    texCoord = vertexTexCoord;
    gl_Position = properties.worldViewProjection * vec4(vertexPosition, 1.0);
}
```

## Verification Steps

### 1. Descriptor Set Layout
```rust
// Created in VkGpuProgram::new_from_shaders()
let bindings = [
    vk::DescriptorSetLayoutBinding {
        binding: 0,
        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        ..Default::default()
    },
    vk::DescriptorSetLayoutBinding {
        binding: 1,
        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        ..Default::default()
    },
];
```

### 2. SPIR-V Compilation
```rust
// Performed by ShaderCompiler::compile_glsl_to_spirv()
let spirv = compiler.compile_into_spirv(
    &processed_source,  // The GLSL 450 code above
    shaderc::ShaderKind::Vertex,
    "blit.shader",
    "main",
    None,
)?;
```

### 3. Shader Module Creation
```rust
// Created in VkGpuShader::new()
let create_info = vk::ShaderModuleCreateInfo::default()
    .code(&spirv);

let module = device.device
    .create_shader_module(&create_info, None)?;
```

## Expected Results

✅ **Compilation**: No errors, clean SPIR-V bytecode
✅ **Validation**: No validation layer warnings
✅ **Rendering**: Texture correctly blitted to screen
✅ **Performance**: Comparable to OpenGL backend

## Testing Commands

```bash
# Compile the Vulkan backend
cargo check -p fyrox-graphics-vk

# Run with validation layers enabled
RUST_LOG=debug cargo run --example 2d

# Enable Vulkan validation
export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
```

## Common Issues and Solutions

### Issue 1: Binding Mismatch
**Symptom**: Validation error about descriptor set mismatch
**Solution**: Ensure bindings in shader match VkDescriptorSetLayoutBinding

### Issue 2: Uniform Buffer Alignment
**Symptom**: Incorrect uniform values
**Solution**: Verify std140 layout rules are followed

### Issue 3: Missing Shared Functions
**Symptom**: Compilation error about undefined function
**Solution**: Verify shared.glsl is correctly included in preamble

## Success Criteria

- [ ] Shader compiles to SPIR-V without errors
- [ ] Validation layers report no issues
- [ ] Descriptor sets bind correctly
- [ ] Rendering output is correct
- [ ] Performance is acceptable

## Next Test: Complex Shader

Try with a more complex shader like `deferred_point_light.shader`:
- Multiple textures (4+)
- Uniform buffer with many properties
- Cube map sampler
- PBR lighting calculations using shared functions

---

**Status**: Ready for runtime testing with actual Vulkan backend
