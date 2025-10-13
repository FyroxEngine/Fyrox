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
use crate::memory::{VkBuffer, VkMemoryManager};
use ash::vk;
use fyrox_graphics::{
    core::math::Rect, error::FrameworkError, framebuffer::GpuFrameBufferTrait,
    read_buffer::GpuAsyncReadBufferTrait,
};
use gpu_allocator::MemoryLocation;
use std::rc::Rc;
use std::sync::Arc;

/// Vulkan async read buffer implementation.
pub struct VkGpuAsyncReadBuffer {
    /// The underlying buffer.
    buffer: VkBuffer,
    /// Device reference.
    #[allow(dead_code)]
    device: Arc<VkDevice>,
}

impl VkGpuAsyncReadBuffer {
    /// Creates a new Vulkan async read buffer.
    pub fn new(
        memory_manager: Arc<VkMemoryManager>,
        name: &str,
        pixel_size: usize,
        pixel_count: usize,
    ) -> Result<Self, FrameworkError> {
        let buffer_size = (pixel_size * pixel_count) as vk::DeviceSize;

        let buffer = memory_manager
            .create_buffer(
                buffer_size,
                vk::BufferUsageFlags::TRANSFER_DST,
                MemoryLocation::GpuToCpu,
                name,
            )
            .map_err(|e| FrameworkError::Custom(format!("Failed to create read buffer: {}", e)))?;

        Ok(Self {
            buffer,
            device: memory_manager.device().clone(),
        })
    }

    /// Gets the Vulkan buffer handle.
    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer.buffer
    }
}

impl GpuAsyncReadBufferTrait for VkGpuAsyncReadBuffer {
    fn schedule_pixels_transfer(
        &self,
        _framebuffer: &dyn GpuFrameBufferTrait,
        _color_buffer_index: u32,
        _rect: Option<Rect<i32>>,
    ) -> Result<(), FrameworkError> {
        // This would need to be implemented to schedule a transfer from source to this buffer
        // using command buffers. For now, just return success.
        Ok(())
    }

    fn is_request_running(&self) -> bool {
        // For now, assume no request is running
        false
    }

    fn try_read(&self) -> Option<Vec<u8>> {
        // For now, return None as no data is available
        None
    }
}

/// Creates a Vulkan async read buffer.
pub fn create_async_read_buffer(
    memory_manager: Arc<VkMemoryManager>,
    name: &str,
    pixel_size: usize,
    pixel_count: usize,
) -> Result<Rc<dyn GpuAsyncReadBufferTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuAsyncReadBuffer::new(
        memory_manager,
        name,
        pixel_size,
        pixel_count,
    )?))
}
