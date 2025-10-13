# Agent Instructions for Fyrox Game Engine Development

## Current Version Information
- **Fyrox Version**: 1.0.0-rc.1 (Release Candidate 1)
- **Rust Version**: 1.86 (minimum required)
- **Edition**: 2021

## Core Principles

### 1. API Accuracy and Verification
**CRITICAL**: Always verify API existence before suggesting or using any Fyrox API components.

- **DO NOT** invent or assume the existence of structs, methods, or modules
- **ALWAYS** check existing code patterns in the codebase before suggesting solutions
- **VERIFY** method signatures, parameter types, and return types from actual source code
- **USE** semantic search and file reading tools to validate API usage patterns

### 2. Version-Specific Considerations

#### Current API Structure (v1.0.0-rc.1)
- Main engine entry point: `fyrox-impl` crate (implementation)
- Re-exported through `fyrox` crate
- Core modules: `fyrox_core`, `fyrox_ui`, `fyrox_resource`, `fyrox_graphics`, etc.
- Physics: Rapier3D (v0.29) and Rapier2D (v0.29) integration

#### Engine Initialization Pattern
```rust
use fyrox::engine::{Engine, EngineInitParams, GraphicsContextParams, Executor};
use fyrox::event_loop::EventLoop;

// Modern initialization using Executor
let mut executor = Executor::from_params(
    EventLoop::new().unwrap(),
    GraphicsContextParams {
        window_attributes: Default::default(),
        vsync: true,
        msaa_sample_count: None,
    },
);
executor.add_plugin_constructor(GameConstructor);
executor.run();
```

### 3. Scene Graph and Node System

#### Current Node Architecture
- All nodes derive from `Base` struct
- Use `NodeTrait` for custom behavior
- Scene graph is managed by `Graph` structure
- Handle-based node references: `Handle<Node>`

#### Physics Integration (Updated in v1.0.0)
**IMPORTANT**: Physics system was completely redesigned. Current structure:

```rust
// Rigid Body as parent, children are visual/collider components
- RigidBody (scene::rigidbody::RigidBody)
  - Visual mesh/model
  - Collider (scene::collider::Collider)
```

**Properties of RigidBody node:**
- `lin_vel: InheritableVariable<Vector3<f32>>`
- `ang_vel: InheritableVariable<Vector3<f32>>`
- `lin_damping: InheritableVariable<f32>`
- `ang_damping: InheritableVariable<f32>`
- `body_type: InheritableVariable<RigidBodyType>`
- `mass: InheritableVariable<f32>`
- `ccd_enabled: InheritableVariable<bool>`
- `can_sleep: InheritableVariable<bool>`

### 4. Plugin System

#### Current Plugin Architecture
```rust
use fyrox::plugin::{Plugin, PluginContext, PluginRegistrationContext};

#[derive(Default, Visit, Reflect, Debug)]
pub struct GamePlugin {
    scene: Handle<Scene>,
}

impl Plugin for GamePlugin {
    fn register(&self, context: PluginRegistrationContext) {
        // Register scripts and custom nodes
    }

    fn init(&mut self, scene_path: Option<&str>, context: PluginContext) {
        // Initialize plugin
    }

    fn update(&mut self, context: &mut PluginContext) {
        // Update logic
    }

    fn on_os_event(&mut self, event: &Event<()>, context: PluginContext) {
        // Handle OS events
    }
}
```

### 5. Resource Management

#### Current Resource System
- `ResourceManager` for asset loading
- Async loading through `AsyncSceneLoader`
- Resource types: textures, models, scenes, sounds, etc.
- Path-based resource loading with `.rgs` scene files

### 6. Graphics Context

#### Modern Graphics Initialization
- Graphics context created on `Event::Resumed`
- Destroyed on `Event::Suspended`
- OpenGL 3.3 Core / OpenGL ES 3.0 minimum
- Uses `winit` v0.30 for windowing

### 7. Common Patterns to Follow

#### Error Handling
- Use `Result<T, EngineError>` for engine operations
- `Result<T, FrameworkError>` for graphics operations
- Proper error propagation with `?` operator

#### Transform Operations
```rust
// Setting transform properties
node.local_transform_mut()
    .set_position(position)
    .set_rotation(rotation)
    .set_scale(scale);

// Global transform access
let global_transform = node.global_transform();
```

#### Scene Management
```rust
// Adding nodes to scene
let handle = scene.graph.add_node(Node::new(base));

// Node queries
if let Some(node) = scene.graph.try_get(handle) {
    // Use node
}
```

### 8. Module Structure Verification

#### Always Check Module Paths
- `fyrox::scene::*` - Scene graph components
- `fyrox::engine::*` - Engine core
- `fyrox::resource::*` - Resource management (aliased as `asset`)
- `fyrox::gui::*` - UI system (aliased from `fyrox_ui`)
- `fyrox::core::*` - Core utilities
- `fyrox::graphics::*` - Graphics abstractions

### 9. Dependencies and Features

#### Current Dependencies (verify before use)
- `rapier3d = "0.29"` (3D physics)
- `rapier2d = "0.29"` (2D physics)
- `winit = "0.30"` (windowing)
- `image = "0.25.1"` (image loading)
- `serde = "1"` (serialization)
- `ron = "0.11.0"` (resource format)

### 10. Code Generation Guidelines

#### Before Writing Any Code:
1. **Search** existing patterns in the codebase
2. **Read** relevant source files to understand current API
3. **Verify** all struct names, method signatures, and imports
4. **Check** for deprecated patterns or outdated examples
5. **Validate** against the actual module structure

#### When Suggesting Solutions:
- Provide working examples based on current API
- Include proper imports and module paths
- Mention any version-specific considerations
- Reference actual source code when possible
- Warn about any assumptions or unverified patterns

#### Testing Approach:
- Always suggest testing new code incrementally
- Provide compilation checks for type compatibility
- Include error handling appropriate to the context
- Consider platform-specific behavior (WASM, mobile, desktop)

### 11. Red Flags - Things to Avoid

- **DON'T** use outdated physics API (pre-v1.0 physics nodes)
- **DON'T** assume method existence without verification
- **DON'T** use hardcoded version numbers in suggested code
- **DON'T** ignore the Handle<T> system for node references
- **DON'T** forget about the async nature of resource loading
- **DON'T** mix 2D and 3D physics APIs incorrectly

### 12. When Uncertain

If you're unsure about any API usage:
1. **STOP** and search the codebase first
2. **READ** relevant source files
3. **ASK** the user for clarification if needed
4. **PROVIDE** a disclaimer about any assumptions
5. **SUGGEST** consulting the official documentation

Remember: It's better to ask for clarification than to provide incorrect code that won't compile or work with the current Fyrox version.

---

## 13. Vulkan Backend: Understanding and Problem-Solving

### Overview
Fyrox includes an experimental Vulkan graphics backend (`fyrox-graphics-vk`) as an alternative to the default OpenGL backend. Understanding how Vulkan works and common issues is critical for debugging and development.

### 13.1 Vulkan Backend Architecture

#### Key Components
The Vulkan backend is organized into modular components:

```
fyrox-graphics-vk/
├── src/
│   ├── lib.rs           # Main exports and utilities
│   ├── server.rs        # VkGraphicsServer (main entry point)
│   ├── instance.rs      # Vulkan instance and validation layers
│   ├── device.rs        # Logical device and queue management
│   ├── swapchain.rs     # Swapchain for window rendering
│   ├── program.rs       # Shader compilation (GLSL → SPIR-V)
│   ├── texture.rs       # Texture/image handling
│   ├── buffer.rs        # Vertex/index/uniform buffers
│   ├── framebuffer.rs   # Render targets and attachments
│   ├── command.rs       # Command buffer management
│   ├── memory.rs        # GPU memory allocation (gpu-allocator)
│   └── ...
```

#### Dependencies (Cargo.toml)
```toml
ash = "0.38"                      # Vulkan bindings
gpu-allocator = "0.27"            # Memory management
shaderc = "0.8"                   # GLSL → SPIR-V compilation
spirv-reflect = "0.2"             # SPIR-V reflection
ash-window = "0.13"               # Surface creation
winit = "0.30"                    # Window integration
```

#### Main Entry Point
```rust
// VkGraphicsServer::new() creates the full Vulkan stack:
pub fn new(
    _vsync: bool,
    _msaa_sample_count: Option<u8>,
    window_target: &ActiveEventLoop,
    window_attributes: WindowAttributes,
    _named_objects: bool,
) -> Result<(Window, SharedGraphicsServer), FrameworkError>
```

### 13.2 Vulkan Initialization Flow

#### Step-by-Step Initialization
1. **Create Window** (via `winit`)
2. **Create VkInstance** with extensions and validation layers
3. **Create Surface** (platform-specific via `ash-window`)
4. **Select Physical Device** (GPU)
5. **Create Logical Device** with queues
6. **Create Memory Manager** (`gpu-allocator`)
7. **Create Command Manager** for command buffer pools
8. **Create Swapchain** for presenting to window
9. **Create Back Buffer Framebuffer**

#### Critical Resource Ordering
**IMPORTANT**: Vulkan resources MUST be destroyed in reverse order of creation!

```rust
// VkGraphicsServer uses ManuallyDrop to control destruction order:
pub struct VkGraphicsServer {
    instance: std::mem::ManuallyDrop<VkInstance>,          // Drop LAST
    device: std::mem::ManuallyDrop<Arc<VkDevice>>,         // Drop before instance
    memory_manager: std::mem::ManuallyDrop<Arc<VkMemoryManager>>,
    command_manager: std::mem::ManuallyDrop<Arc<CommandManager>>,
    swapchain: Option<Arc<Mutex<VkSwapchain>>>,            // Drop FIRST
    // ...
}

impl Drop for VkGraphicsServer {
    fn drop(&mut self) {
        // Manual drop in correct order (reverse of creation)
        unsafe {
            // 1. Drop swapchain first
            drop(self.swapchain.take());
            // 2. Drop command manager
            std::mem::ManuallyDrop::drop(&mut self.command_manager);
            // 3. Drop memory manager
            std::mem::ManuallyDrop::drop(&mut self.memory_manager);
            // 4. Drop device
            std::mem::ManuallyDrop::drop(&mut self.device);
            // 5. Drop instance last
            std::mem::ManuallyDrop::drop(&mut self.instance);
        }
    }
}
```

### 13.3 Validation Layers and Debugging

#### Validation Layer Setup
```rust
// Automatically enabled in debug builds
#[cfg(debug_assertions)]
let enable_validation = true;

// Creates instance with VK_LAYER_KHRONOS_validation
let instance = VkInstance::new(
    Some(&window),
    Some(&window),
    enable_validation
)?;
```

#### Debug Messenger
The `VkInstance` includes optional debug messenger for validation errors:
```rust
pub struct VkInstance {
    pub entry: Entry,
    pub instance: Instance,
    pub debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    pub debug_utils: Option<ash::ext::debug_utils::Instance>,
}
```

#### Common Validation Errors and Solutions

| Error | Cause | Solution |
|-------|-------|----------|
| `VK_LAYER_KHRONOS_validation not available` | Vulkan SDK not installed or incomplete | Install Vulkan SDK from https://vulkan.lunarg.com/ |
| `VK_ERROR_INITIALIZATION_FAILED` | Missing validation layers | Install SDK with validation layers or disable in release builds |
| `VK_ERROR_DEVICE_LOST` | GPU crash or driver timeout | Check shader code, reduce workload, update drivers |
| `VK_ERROR_OUT_OF_DEVICE_MEMORY` | GPU memory exhausted | Reduce texture sizes, check for memory leaks |
| `VK_ERROR_SURFACE_LOST_KHR` | Window closed or resized improperly | Handle window events correctly |

### 13.4 Shader Compilation (GLSL → SPIR-V)

#### The Shader Pipeline
Fyrox shaders are written in GLSL and compiled to SPIR-V at runtime:

```rust
pub struct ShaderCompiler {
    compiler: shaderc::Compiler,
}

impl ShaderCompiler {
    pub fn compile_glsl_to_spirv(
        &mut self,
        source: &str,
        shader_kind: &ShaderKind,
        input_file_name: &str,
        entry_point_name: &str,
        additional_options: Option<&shaderc::CompileOptions>,
    ) -> Result<Vec<u32>, FrameworkError>
```

#### Common Shader Compilation Errors

1. **GLSL Version Mismatch**
   ```glsl
   // ❌ Wrong - OpenGL syntax
   #version 330 core

   // ✅ Correct - Vulkan-compatible GLSL
   #version 450
   ```

2. **Binding Declarations**
   ```glsl
   // ❌ Wrong - Missing layout qualifiers
   uniform sampler2D myTexture;

   // ✅ Correct - Explicit bindings for Vulkan
   layout(binding = 0) uniform sampler2D myTexture;
   ```

3. **Descriptor Sets**
   ```glsl
   // Vulkan requires explicit set and binding
   layout(set = 0, binding = 0) uniform UniformBuffer {
       mat4 projection;
       mat4 view;
   };
   ```

#### SPIR-V Reflection
After compilation, `spirv-reflect` analyzes the bytecode to extract:
- Uniform buffer layouts
- Texture bindings
- Push constant ranges
- Input/output variables

### 13.5 Common Compilation Errors

#### 1. Type Mismatches (HashMap<ImmutableString, T> vs HashMap<String, T>)

**Error in errors.txt:**
```
error[E0308]: mismatched types
   --> fyrox-graphics-vk\src\program.rs:296:13
    |
296 |             uniform_locations,
    |             ^^^^^^^^^^^^^^^^^ expected `HashMap<String, usize>`,
    |                               found `HashMap<ImmutableString, usize>`
```

**Cause**: The `ShaderResourceDefinition.name` field is `ImmutableString`, but the API expects `HashMap<String, usize>`.

**Solution**: Convert to `String` when inserting:
```rust
// ❌ Wrong
uniform_locations.insert(resource.name.clone(), index);

// ✅ Correct
uniform_locations.insert(resource.name.to_string(), index);
```

#### 2. Unused Imports/Variables

**Warnings**: The compiler shows many unused imports/variables during initial development.

**Solution Strategy**:
- Use `#[allow(unused)]` temporarily during prototyping
- Remove unused imports systematically
- Prefix intentionally unused variables with `_` (e.g., `_window`)

```rust
// Suppress warnings for work-in-progress code
#[allow(unused_imports)]
use fyrox_graphics::DrawParameters;

// Or prefix unused parameters
fn create_buffer(&self, _capacity: usize) { }
```

### 13.6 Runtime Issues and Debugging

#### Memory Management

**GPU Allocator Setup:**
```rust
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};

pub struct VkMemoryManager {
    allocator: Arc<Mutex<Allocator>>,
    device: Arc<VkDevice>,
}

impl VkMemoryManager {
    pub fn new(device: Arc<VkDevice>, instance: &Instance)
        -> Result<Self, FrameworkError>
```

**Common Memory Issues:**
1. **Memory Leaks**: Forgetting to free allocations
   - Always pair `allocate()` with `free()`
   - Use RAII patterns (Drop trait)

2. **Fragmentation**: Many small allocations
   - Use sub-allocation for small buffers
   - Pool frequently allocated resources

3. **Wrong Memory Type**: Using CPU-visible memory for GPU-only data
   - Use `MemoryLocation::GpuOnly` for static data
   - Use `MemoryLocation::CpuToGpu` for dynamic updates

#### Synchronization Issues

**The Golden Rule**: Vulkan does NOT synchronize automatically!

Common synchronization bugs:
```rust
// ❌ Wrong - No synchronization
command_buffer.copy_buffer(src, dst);
command_buffer.draw(...); // May read from dst before copy finishes!

// ✅ Correct - Add pipeline barrier
command_buffer.copy_buffer(src, dst);
command_buffer.pipeline_barrier(
    vk::PipelineStageFlags::TRANSFER,
    vk::PipelineStageFlags::VERTEX_INPUT,
    barrier
);
command_buffer.draw(...);
```

**Synchronization Primitives:**
- **Semaphores**: GPU-GPU synchronization (e.g., between queues)
- **Fences**: CPU-GPU synchronization (e.g., waiting for frame completion)
- **Pipeline Barriers**: In-command-buffer synchronization
- **Events**: Fine-grained GPU synchronization

#### Swapchain and Presentation

**Swapchain Invalidation**: Occurs on window resize or minimize.

```rust
// Handle VK_ERROR_OUT_OF_DATE_KHR and VK_SUBOPTIMAL_KHR
match swapchain.acquire_next_image() {
    Ok((image_index, is_suboptimal)) => {
        if is_suboptimal {
            // Recreate swapchain
        }
    }
    Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
        // Recreate swapchain
    }
    Err(e) => return Err(e),
}
```

### 13.7 Performance Optimization

#### Vulkan Best Practices

1. **Batch Draw Calls**: Minimize state changes
   ```rust
   // ❌ Bad - Many state changes
   for mesh in meshes {
       bind_pipeline(mesh.material);
       bind_descriptor_set(mesh.descriptors);
       draw(mesh);
   }

   // ✅ Good - Sort by state
   meshes.sort_by(|a, b| a.material.id.cmp(&b.material.id));
   let mut current_material = None;
   for mesh in meshes {
       if Some(mesh.material.id) != current_material {
           bind_pipeline(mesh.material);
           current_material = Some(mesh.material.id);
       }
       bind_descriptor_set(mesh.descriptors);
       draw(mesh);
   }
   ```

2. **Use Descriptor Pools**: Pre-allocate descriptor sets
3. **Command Buffer Reuse**: Record once, submit multiple times
4. **Transfer Queue**: Use dedicated transfer queue for large uploads
5. **Staging Buffers**: For CPU→GPU transfers

#### Profiling Tools
- **RenderDoc**: Capture and analyze frames
- **Vulkan Validation Layers**: Check for errors
- **GPU Vendor Tools**:
  - NVIDIA Nsight Graphics
  - AMD Radeon GPU Profiler
  - Intel GPA

### 13.8 Testing the Vulkan Backend

#### Feature Flag
Enable Vulkan backend with feature flag:
```toml
# In Cargo.toml
[features]
default = ["fyrox-graphics-gl"]
vulkan = ["fyrox-graphics-vk"]
```

#### Build and Test Commands
```powershell
# Check compilation
cargo check -p fyrox-graphics-vk

# Build with warnings
cargo build -p fyrox-graphics-vk

# Run specific example with Vulkan
cargo run --example 2d --features vulkan

# Run editor with Vulkan
cargo run --bin fyroxed --features vulkan
```

#### Debugging Compilation Errors
```powershell
# Capture errors to file
cargo check -p fyrox-graphics-vk 2> errors.txt

# Show full error context
cargo check -p fyrox-graphics-vk --message-format=json

# Check specific file
cargo check -p fyrox-graphics-vk --lib
```

### 13.9 Vulkan-Specific Troubleshooting Checklist

When encountering Vulkan issues, systematically check:

#### ✅ Installation and Environment
- [ ] Vulkan SDK installed (version 1.3+)
- [ ] Graphics drivers up to date
- [ ] `VK_LAYER_PATH` environment variable set (if needed)
- [ ] Vulkan loader DLL/SO available

#### ✅ Code Compilation
- [ ] All imports resolved (`use` statements correct)
- [ ] Type conversions match (String vs ImmutableString)
- [ ] Feature flags enabled in Cargo.toml
- [ ] Dependencies versions match Cargo.lock

#### ✅ Runtime Issues
- [ ] Validation layers enabled for debugging
- [ ] GPU supports required Vulkan features
- [ ] Surface format and present mode supported
- [ ] Memory allocations succeed (not out of memory)
- [ ] Swapchain recreated on window resize
- [ ] Command buffers synchronized correctly
- [ ] Resources destroyed in correct order

#### ✅ Shader Issues
- [ ] GLSL version is 450+ (Vulkan-compatible)
- [ ] All bindings explicitly declared
- [ ] Descriptor set layouts match shader expectations
- [ ] SPIR-V compilation succeeds
- [ ] Entry point name is "main"

### 13.10 Quick Reference: Common Vulkan Patterns

#### Creating a Buffer
```rust
use fyrox_graphics::buffer::{GpuBuffer, GpuBufferDescriptor, GpuBufferKind};

let buffer = server.create_buffer(GpuBufferDescriptor {
    element_size: std::mem::size_of::<Vertex>(),
    element_count: vertices.len(),
    kind: GpuBufferKind::StaticDraw,
})?;
```

#### Creating a Texture
```rust
use fyrox_graphics::gpu_texture::{GpuTexture, GpuTextureDescriptor, GpuTextureKind};

let texture = server.create_texture(GpuTextureDescriptor {
    kind: GpuTextureKind::Rectangle { width: 512, height: 512 },
    format: PixelFormat::RGBA8,
    min_filter: MinificationFilter::Linear,
    mag_filter: MagnificationFilter::Linear,
    mip_count: 1,
    s_wrap_mode: WrapMode::ClampToEdge,
    t_wrap_mode: WrapMode::ClampToEdge,
})?;
```

#### Shader Compilation
```rust
use fyrox_graphics::gpu_program::{GpuShader, ShaderKind};

let vertex_shader = server.create_shader(
    "MyShader",
    ShaderKind::Vertex,
    vertex_source,
    &resources,
    0,
)?;
```

### 13.11 Resources and Documentation

#### Official Documentation
- **Vulkan Specification**: https://registry.khronos.org/vulkan/
- **Vulkan Tutorial**: https://vulkan-tutorial.com/
- **Ash (Rust Bindings)**: https://docs.rs/ash/

#### Fyrox-Specific
- **fyrox-graphics**: Abstract graphics API traits
- **fyrox-graphics-vk**: Vulkan implementation
- **VULKAN_TESTING.md**: Testing guide (in repo)

#### Community Resources
- **Vulkan Discord**: https://discord.gg/vulkan
- **Rust Graphics Discord**: https://discord.gg/rust-gamedev

---

### Summary: Vulkan Problem-Solving Strategy

1. **Identify the Error Category**: Compilation, linking, runtime, or validation
2. **Check Prerequisites**: SDK, drivers, environment
3. **Verify Code Structure**: Imports, types, API usage
4. **Enable Validation Layers**: Get detailed error messages
5. **Consult This Guide**: Match error patterns to solutions
6. **Search the Codebase**: Look for working examples
7. **Test Incrementally**: Build components one at a time

**Remember**: Vulkan is explicit and verbose by design. Every error message contains useful information—read them carefully!
