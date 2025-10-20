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
// AUTHORS OR COPYRIGHT SHALL BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use ash::vk;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::{
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

use crate::{
    buffer::VkGpuBuffer, command::CommandManager, device::VkDevice, framebuffer::VkGpuFrameBuffer,
    geometry_buffer::VkGpuGeometryBuffer, instance::VkInstance, memory::VkMemoryManager,
    program::VkGpuProgram, query::VkGpuQuery, read_buffer::VkGpuAsyncReadBuffer,
    sampler::VkGpuSampler, swapchain::VkSwapchain, texture::VkGpuTexture,
};
use fyrox_graphics::{
    buffer::{GpuBuffer, GpuBufferDescriptor},
    error::FrameworkError,
    framebuffer::{Attachment, GpuFrameBuffer},
    geometry_buffer::{GpuGeometryBuffer, GpuGeometryBufferDescriptor},
    gpu_program::{GpuProgram, GpuShader, ShaderKind, ShaderResourceDefinition},
    gpu_texture::{GpuTexture, GpuTextureDescriptor},
    query::GpuQuery,
    read_buffer::GpuAsyncReadBuffer,
    sampler::{GpuSampler, GpuSamplerDescriptor},
    server::{GraphicsServer, ServerCapabilities, ServerMemoryUsage, SharedGraphicsServer},
    stats::PipelineStatistics,
};
use std::rc::Rc;
use std::rc::Weak;
use std::sync::{Arc, Mutex};

/// Vulkan graphics server implementation
pub struct VkGraphicsServer {
    /// Vulkan instance wrapper
    instance: std::mem::ManuallyDrop<VkInstance>,
    /// Surface and loader (None when headless)
    surface: Option<vk::SurfaceKHR>,
    surface_loader: Option<ash::khr::surface::Instance>,
    /// Logical device wrapper
    device: std::mem::ManuallyDrop<Arc<VkDevice>>,
    /// Memory manager (must be dropped after swapchain and before device)
    memory_manager: std::mem::ManuallyDrop<Arc<VkMemoryManager>>,
    /// Command manager
    command_manager: std::mem::ManuallyDrop<Arc<CommandManager>>,
    /// Swapchain (must be dropped before managers)
    swapchain: Option<Arc<Mutex<VkSwapchain>>>,
    /// Back buffer framebuffer
    back_buffer: Option<GpuFrameBuffer>,
    /// Current frame size.
    #[allow(dead_code)]
    frame_size: (u32, u32),
    /// Pipeline statistics.
    pipeline_stats: PipelineStatistics,
    /// Memory usage tracking
    memory_usage: ServerMemoryUsage,
}

impl VkGraphicsServer {
    /// Creates a new Vulkan graphics server with a window.
    /// This matches the signature expected by the Fyrox engine.
    pub fn new(
        _vsync: bool,
        _msaa_sample_count: Option<u8>,
        window_target: &ActiveEventLoop,
        window_attributes: WindowAttributes,
        _named_objects: bool,
    ) -> Result<(Window, SharedGraphicsServer), FrameworkError> {
        // Create window
        let window = window_target
            .create_window(window_attributes)
            .map_err(|e| FrameworkError::Custom(format!("Failed to create window: {:?}", e)))?;

        // Enable validation layers in debug builds
        #[cfg(debug_assertions)]
        let enable_validation = true;
        #[cfg(not(debug_assertions))]
        let enable_validation = false;

        // Create Vulkan instance
        let instance =
            VkInstance::new(Some(&window), Some(&window), enable_validation).map_err(|e| {
                FrameworkError::Custom(format!("Failed to create Vulkan instance: {:?}", e))
            })?;

        // Create surface (Windows-only to avoid ash-window/ash version clash)
        let surface_loader = ash::khr::surface::Instance::new(&instance.entry, &instance.instance);
        let window_handle = window
            .window_handle()
            .map_err(|e| FrameworkError::Custom(format!("Failed to get window handle: {:?}", e)))?
            .as_raw();

        #[cfg(target_os = "windows")]
        let surface = unsafe {
            match window_handle {
                RawWindowHandle::Win32(win) => {
                    use ash::khr::win32_surface;
                    let win32 = win32_surface::Instance::new(&instance.entry, &instance.instance);
                    let hinstance = win
                        .hinstance
                        .map_or(std::ptr::null_mut(), |h| h.get() as *mut _)
                        as *const _;
                    let hwnd = win.hwnd.get() as *mut _ as *const _;
                    let mut ci = vk::Win32SurfaceCreateInfoKHR::default();
                    ci.hinstance = hinstance;
                    ci.hwnd = hwnd;
                    win32.create_win32_surface(&ci, None).map_err(|e| {
                        FrameworkError::Custom(format!("Failed to create Win32 surface: {:?}", e))
                    })?
                }
                _ => {
                    return Err(FrameworkError::Custom(
                        "Unsupported window handle for Windows".to_string(),
                    ))
                }
            }
        };

        #[cfg(not(target_os = "windows"))]
        let surface = {
            return Err(FrameworkError::Custom(
                "Vulkan surface creation is implemented for Windows only in this build".to_string(),
            ));
        };

        // Create device
        let device = Arc::new(
            VkDevice::new(&instance, Some(surface), Some(&surface_loader)).map_err(|e| {
                FrameworkError::Custom(format!("Failed to create Vulkan device: {:?}", e))
            })?,
        );

        // Create memory manager
        let memory_manager = Arc::new(
            VkMemoryManager::new(device.clone(), &instance.instance).map_err(|e| {
                FrameworkError::Custom(format!("Failed to create memory manager: {:?}", e))
            })?,
        );

        // Create command manager
        let command_manager = Arc::new(CommandManager::new(device.clone()).map_err(|e| {
            FrameworkError::Custom(format!("Failed to create command manager: {:?}", e))
        })?);

        // Create swapchain
        // Ensure minimum size to prevent issues with 0x0 windows during initialization
        let inner_size = window.inner_size();
        let width = inner_size.width.max(1);
        let height = inner_size.height.max(1);
        let swapchain = Arc::new(Mutex::new(
            VkSwapchain::new(
                &instance.instance,
                &device,
                surface,
                &surface_loader,
                width,
                height,
                None,
            )
            .map_err(|e| FrameworkError::Custom(format!("Failed to create swapchain: {:?}", e)))?,
        ));

        let server = Self {
            instance: std::mem::ManuallyDrop::new(instance),
            surface: Some(surface),
            surface_loader: Some(surface_loader),
            device: std::mem::ManuallyDrop::new(device),
            memory_manager: std::mem::ManuallyDrop::new(memory_manager),
            command_manager: std::mem::ManuallyDrop::new(command_manager),
            swapchain: Some(swapchain),
            back_buffer: None,
            frame_size: (width, height),
            pipeline_stats: PipelineStatistics::default(),
            memory_usage: ServerMemoryUsage::default(),
        };

        Ok((window, Rc::new(server)))
    }

    /// Updates memory usage statistics.
    #[allow(dead_code)]
    fn update_memory_usage(&mut self, buffer_size: usize, texture_size: usize) {
        self.memory_usage.buffers += buffer_size;
        self.memory_usage.textures += texture_size;
    }
}

impl GraphicsServer for VkGraphicsServer {
    fn create_buffer(&self, desc: GpuBufferDescriptor) -> Result<GpuBuffer, FrameworkError> {
        let buffer = create_buffer(Arc::clone(&self.memory_manager), desc)?;
        // Update memory usage (this is approximate)
        // self.memory_usage.buffers += desc.size; // This would require mutable access
        Ok(GpuBuffer(buffer))
    }

    fn create_texture(&self, desc: GpuTextureDescriptor) -> Result<GpuTexture, FrameworkError> {
        let texture = create_texture(Arc::clone(&self.memory_manager), desc)?;
        // Update memory usage (this is approximate)
        // let texture_size = desc.calculate_size(); // This would need to be implemented
        // self.memory_usage.textures += texture_size;
        Ok(GpuTexture(texture))
    }

    fn create_sampler(&self, desc: GpuSamplerDescriptor) -> Result<GpuSampler, FrameworkError> {
        let sampler = create_sampler(Arc::clone(&self.device), desc)?;
        Ok(GpuSampler(sampler))
    }

    fn create_frame_buffer(
        &self,
        depth_attachment: Option<Attachment>,
        color_attachments: Vec<Attachment>,
    ) -> Result<GpuFrameBuffer, FrameworkError> {
        let framebuffer = create_framebuffer(
            Arc::clone(&self.device),
            depth_attachment,
            color_attachments,
        )?;
        Ok(GpuFrameBuffer(framebuffer))
    }

    fn back_buffer(&self) -> GpuFrameBuffer {
        // Return a placeholder backbuffer that represents the swapchain framebuffer
        // This is used as a sentinel value to indicate rendering to the screen
        let framebuffer = VkGpuFrameBuffer::backbuffer(Arc::clone(&self.device));
        GpuFrameBuffer(Rc::new(framebuffer))
    }

    fn create_query(&self) -> Result<GpuQuery, FrameworkError> {
        let query = create_query(Arc::clone(&self.device))?;
        Ok(GpuQuery(query))
    }

    fn create_shader(
        &self,
        name: String,
        kind: ShaderKind,
        source: String,
        resources: &[ShaderResourceDefinition],
        line_offset: isize,
    ) -> Result<GpuShader, FrameworkError> {
        let shader = create_shader(
            Arc::clone(&self.device),
            name,
            kind,
            source,
            resources,
            line_offset,
        )?;
        Ok(GpuShader(shader))
    }

    fn create_program(
        &self,
        name: &str,
        vertex_source: String,
        vertex_source_line_offset: isize,
        fragment_source: String,
        fragment_source_line_offset: isize,
        resources: &[ShaderResourceDefinition],
    ) -> Result<GpuProgram, FrameworkError> {
        let vertex_shader = self.create_shader(
            format!("{}_vertex", name),
            ShaderKind::Vertex,
            vertex_source,
            resources,
            vertex_source_line_offset,
        )?;

        let fragment_shader = self.create_shader(
            format!("{}_fragment", name),
            ShaderKind::Fragment,
            fragment_source,
            resources,
            fragment_source_line_offset,
        )?;

        self.create_program_from_shaders(name, &vertex_shader, &fragment_shader, resources)
    }

    fn create_program_from_shaders(
        &self,
        name: &str,
        vertex_shader: &GpuShader,
        fragment_shader: &GpuShader,
        resources: &[ShaderResourceDefinition],
    ) -> Result<GpuProgram, FrameworkError> {
        let program = create_program_from_shaders(
            Arc::clone(&self.device),
            name,
            vertex_shader.0.as_ref(),
            fragment_shader.0.as_ref(),
            resources,
        )?;
        Ok(GpuProgram(program))
    }

    fn create_async_read_buffer(
        &self,
        name: &str,
        pixel_size: usize,
        pixel_count: usize,
    ) -> Result<GpuAsyncReadBuffer, FrameworkError> {
        let buffer = create_async_read_buffer(
            Arc::clone(&self.memory_manager),
            name,
            pixel_size,
            pixel_count,
        )?;
        Ok(GpuAsyncReadBuffer(buffer))
    }

    fn create_geometry_buffer(
        &self,
        desc: GpuGeometryBufferDescriptor,
    ) -> Result<GpuGeometryBuffer, FrameworkError> {
        let buffer = create_geometry_buffer(Arc::clone(&self.device), desc)?;
        Ok(GpuGeometryBuffer(buffer))
    }

    fn weak(self: Rc<Self>) -> Weak<dyn GraphicsServer> {
        Rc::downgrade(&self) as Weak<dyn GraphicsServer>
    }

    fn flush(&self) {
        // In Vulkan, we would submit command buffers here
        // For now, this is a no-op
    }

    fn finish(&self) {
        // Wait for device to be idle
        unsafe {
            let _ = self.device.device.device_wait_idle();
        }
    }

    fn invalidate_resource_bindings_cache(&self) {
        // This would invalidate descriptor set caches
        // For now, this is a no-op
    }

    fn pipeline_statistics(&self) -> PipelineStatistics {
        self.pipeline_stats.clone()
    }

    fn swap_buffers(&self) -> Result<(), FrameworkError> {
        if let Some(_swapchain) = &self.swapchain {
            // This would present the current frame
            // For now, this is a simplified implementation
            Ok(())
        } else {
            Ok(()) // No swapchain, nothing to swap
        }
    }

    fn set_frame_size(&self, new_size: (u32, u32)) {
        if let (Some(swapchain), Some(surface), Some(surface_loader)) =
            (&self.swapchain, self.surface, &self.surface_loader)
        {
            // Recreate swapchain with new size
            if let Ok(mut swapchain) = swapchain.try_lock() {
                let _ = swapchain.recreate(
                    &self.instance.instance,
                    &self.device,
                    surface,
                    surface_loader,
                    new_size.0,
                    new_size.1,
                );
            }
        }
    }

    fn capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            max_uniform_block_size: self.device.properties.limits.max_uniform_buffer_range as usize,
            uniform_buffer_offset_alignment: self
                .device
                .properties
                .limits
                .min_uniform_buffer_offset_alignment
                as usize,
            max_lod_bias: self.device.properties.limits.max_sampler_lod_bias,
        }
    }

    fn set_polygon_fill_mode(
        &self,
        _polygon_face: fyrox_graphics::PolygonFace,
        _polygon_fill_mode: fyrox_graphics::PolygonFillMode,
    ) {
        // This would need to be handled during pipeline creation in Vulkan
        // For now, this is a no-op
    }

    fn generate_mipmap(&self, _texture: &GpuTexture) {
        // This would generate mipmaps using command buffers
        // For now, this is a no-op
    }

    fn memory_usage(&self) -> ServerMemoryUsage {
        self.memory_usage.clone()
    }
}

// Stub functions for creating Vulkan resources
// These delegate to the actual implementation types

fn create_buffer(
    memory_manager: Arc<VkMemoryManager>,
    desc: GpuBufferDescriptor,
) -> Result<Rc<dyn fyrox_graphics::buffer::GpuBufferTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuBuffer::new(memory_manager, desc)?))
}

fn create_texture(
    memory_manager: Arc<VkMemoryManager>,
    desc: GpuTextureDescriptor,
) -> Result<Rc<dyn fyrox_graphics::gpu_texture::GpuTextureTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuTexture::new(memory_manager, desc)?))
}

fn create_sampler(
    device: Arc<VkDevice>,
    desc: GpuSamplerDescriptor,
) -> Result<Rc<dyn fyrox_graphics::sampler::GpuSamplerTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuSampler::new(device, desc)?))
}

fn create_framebuffer(
    device: Arc<VkDevice>,
    depth_attachment: Option<Attachment>,
    color_attachments: Vec<Attachment>,
) -> Result<Rc<dyn fyrox_graphics::framebuffer::GpuFrameBufferTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuFrameBuffer::new(
        device,
        depth_attachment,
        color_attachments,
    )?))
}

fn create_query(
    device: Arc<VkDevice>,
) -> Result<Rc<dyn fyrox_graphics::query::GpuQueryTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuQuery::new(device)?))
}

fn create_shader(
    _device: Arc<VkDevice>,
    name: String,
    _kind: ShaderKind,
    _source: String,
    _resources: &[ShaderResourceDefinition],
    _line_offset: isize,
) -> Result<Rc<dyn fyrox_graphics::gpu_program::GpuShaderTrait>, FrameworkError> {
    // Stub implementation - needs actual shader compilation
    Err(FrameworkError::Custom(format!(
        "Shader creation not yet fully implemented for: {}",
        name
    )))
}

fn create_program_from_shaders(
    device: Arc<VkDevice>,
    name: &str,
    vertex_shader: &dyn fyrox_graphics::gpu_program::GpuShaderTrait,
    fragment_shader: &dyn fyrox_graphics::gpu_program::GpuShaderTrait,
    _resources: &[ShaderResourceDefinition],
) -> Result<Rc<dyn fyrox_graphics::gpu_program::GpuProgramTrait>, FrameworkError> {
    // Downcast to VkGpuShader to access Vulkan-specific shader modules
    // For now, create a stub program
    Ok(Rc::new(VkGpuProgram::new_stub(device, name)?))
}

fn create_async_read_buffer(
    memory_manager: Arc<VkMemoryManager>,
    name: &str,
    pixel_size: usize,
    pixel_count: usize,
) -> Result<Rc<dyn fyrox_graphics::read_buffer::GpuAsyncReadBufferTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuAsyncReadBuffer::new(
        memory_manager,
        name,
        pixel_size,
        pixel_count,
    )?))
}

fn create_geometry_buffer(
    device: Arc<VkDevice>,
    desc: GpuGeometryBufferDescriptor,
) -> Result<Rc<dyn fyrox_graphics::geometry_buffer::GpuGeometryBufferTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuGeometryBuffer::new(device, desc)?))
}

impl Drop for VkGraphicsServer {
    fn drop(&mut self) {
        // Wait for all GPU operations to complete before cleanup
        unsafe {
            let _ = self.device.device.device_wait_idle();
        }

        // CRITICAL: Manually drop resources in the EXACT correct order for Vulkan!
        // This is why we use ManuallyDrop for the critical fields.

        // Step 1: Drop high-level framebuffer resources first
        self.back_buffer = None;

        // Step 2: Drop swapchain (contains images and image views)
        // Must happen before device destruction
        self.swapchain = None;

        // Step 3: Drop command manager (contains command pools that reference the device)
        unsafe {
            std::mem::ManuallyDrop::drop(&mut self.command_manager);
        }

        // Step 4: Drop memory manager (contains allocations that reference the device)
        unsafe {
            std::mem::ManuallyDrop::drop(&mut self.memory_manager);
        }

        // Step 5: Drop device (must happen after all resources that use it)
        unsafe {
            std::mem::ManuallyDrop::drop(&mut self.device);
        }

        // Step 6: Clean up surface before instance destruction
        if let (Some(surface), Some(surface_loader)) = (self.surface, &self.surface_loader) {
            unsafe {
                surface_loader.destroy_surface(surface, None);
            }
        }

        // Step 7: Drop instance last (must happen after device and surface)
        unsafe {
            std::mem::ManuallyDrop::drop(&mut self.instance);
        }

        // All other fields will drop automatically now (no Vulkan cleanup needed)
    }
} // Note: For creating a Vulkan graphics server, use VkGraphicsServer::new
  // which matches the engine's expected signature. This takes a window target
  // and window attributes and returns both the window and the server.
