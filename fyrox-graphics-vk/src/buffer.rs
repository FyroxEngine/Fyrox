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

use crate::memory::{VkBuffer, VkMemoryManager};
use crate::ToVkType;
use ash::vk;
use fyrox_graphics::{
    buffer::{BufferKind, BufferUsage, GpuBufferDescriptor, GpuBufferTrait},
    error::FrameworkError,
};
use gpu_allocator::MemoryLocation;
use std::rc::Rc;
use std::sync::Arc;

impl ToVkType<vk::BufferUsageFlags> for BufferUsage {
    fn to_vk(self) -> vk::BufferUsageFlags {
        match self {
            BufferUsage::StaticDraw => vk::BufferUsageFlags::VERTEX_BUFFER,
            BufferUsage::StaticRead => vk::BufferUsageFlags::TRANSFER_DST,
            BufferUsage::StaticCopy => {
                vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST
            }
            BufferUsage::DynamicDraw => vk::BufferUsageFlags::VERTEX_BUFFER,
            BufferUsage::DynamicRead => vk::BufferUsageFlags::TRANSFER_DST,
            BufferUsage::DynamicCopy => {
                vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST
            }
            BufferUsage::StreamDraw => vk::BufferUsageFlags::VERTEX_BUFFER,
            BufferUsage::StreamRead => vk::BufferUsageFlags::TRANSFER_DST,
            BufferUsage::StreamCopy => {
                vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST
            }
        }
    }
}

impl ToVkType<vk::BufferUsageFlags> for BufferKind {
    fn to_vk(self) -> vk::BufferUsageFlags {
        match self {
            BufferKind::Vertex => {
                vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST
            }
            BufferKind::Index => {
                vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST
            }
            BufferKind::Uniform => {
                vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST
            }
            BufferKind::PixelRead => vk::BufferUsageFlags::TRANSFER_SRC,
            BufferKind::PixelWrite => vk::BufferUsageFlags::TRANSFER_DST,
        }
    }
}

/// Vulkan buffer implementation.
pub struct VkGpuBuffer {
    /// The underlying Vulkan buffer.
    buffer: VkBuffer,
    /// Buffer kind.
    kind: BufferKind,
    /// Buffer usage.
    usage: BufferUsage,
    /// Memory manager reference.
    memory_manager: Arc<VkMemoryManager>,
}

impl VkGpuBuffer {
    /// Creates a new Vulkan GPU buffer.
    pub fn new(
        memory_manager: Arc<VkMemoryManager>,
        desc: GpuBufferDescriptor,
    ) -> Result<Self, FrameworkError> {
        let usage_flags = desc.kind.to_vk() | desc.usage.to_vk();

        // Determine memory location based on usage
        let memory_location = match desc.usage {
            BufferUsage::DynamicDraw
            | BufferUsage::DynamicRead
            | BufferUsage::DynamicCopy
            | BufferUsage::StreamDraw
            | BufferUsage::StreamRead
            | BufferUsage::StreamCopy => MemoryLocation::CpuToGpu,
            _ => MemoryLocation::GpuOnly,
        };

        let buffer = memory_manager
            .create_buffer(
                desc.size as vk::DeviceSize,
                usage_flags,
                memory_location,
                &desc.name,
            )
            .map_err(|e| FrameworkError::Custom(format!("Failed to create buffer: {}", e)))?;

        Ok(Self {
            buffer,
            kind: desc.kind,
            usage: desc.usage,
            memory_manager,
        })
    }

    /// Gets the Vulkan buffer handle.
    pub fn vk_buffer(&self) -> vk::Buffer {
        self.buffer.buffer
    }

    /// Gets the buffer size.
    pub fn size(&self) -> vk::DeviceSize {
        self.buffer.size
    }
}

impl GpuBufferTrait for VkGpuBuffer {
    fn write_data(&self, data: &[u8]) -> Result<(), FrameworkError> {
        if data.len() > self.buffer.size as usize {
            return Err(FrameworkError::Custom(
                "Data size exceeds buffer size".to_string(),
            ));
        }

        if let Some(mapped_ptr) = self.buffer.map() {
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), mapped_ptr, data.len());
            }
            Ok(())
        } else {
            // For GPU-only buffers, we need to use a staging buffer
            // This is a simplified implementation - in practice, you'd want to batch these operations
            let staging_buffer = self
                .memory_manager
                .create_buffer(
                    data.len() as vk::DeviceSize,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    MemoryLocation::CpuToGpu,
                    "staging_buffer",
                )
                .map_err(|e| {
                    FrameworkError::Custom(format!("Failed to create staging buffer: {}", e))
                })?;

            staging_buffer.copy_from_slice(data).map_err(|e| {
                FrameworkError::Custom(format!("Failed to copy to staging buffer: {}", e))
            })?;

            // TODO: Copy from staging buffer to GPU buffer using command buffer
            // This would require access to command manager

            Err(FrameworkError::Custom(
                "GPU-only buffer write not implemented yet".to_string(),
            ))
        }
    }

    fn read_data(&self, data: &mut [u8]) -> Result<(), FrameworkError> {
        if data.len() > self.buffer.size as usize {
            return Err(FrameworkError::Custom(
                "Data size exceeds buffer size".to_string(),
            ));
        }

        if let Some(mapped_ptr) = self.buffer.map() {
            unsafe {
                std::ptr::copy_nonoverlapping(mapped_ptr, data.as_mut_ptr(), data.len());
            }
            Ok(())
        } else {
            Err(FrameworkError::Custom(
                "Buffer is not readable from CPU".to_string(),
            ))
        }
    }

    fn size(&self) -> usize {
        self.buffer.size as usize
    }

    fn kind(&self) -> BufferKind {
        self.kind
    }

    fn usage(&self) -> BufferUsage {
        self.usage
    }
}

/// Creates a Vulkan GPU buffer.
pub fn create_buffer(
    memory_manager: Arc<VkMemoryManager>,
    desc: GpuBufferDescriptor,
) -> Result<Rc<dyn GpuBufferTrait>, FrameworkError> {
    Ok(Rc::new(VkGpuBuffer::new(memory_manager, desc)?))
}
