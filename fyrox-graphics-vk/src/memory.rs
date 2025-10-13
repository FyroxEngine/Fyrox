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
use ash::vk;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use gpu_allocator::{vulkan::AllocationCreateDesc, MemoryLocation};
use std::sync::{Arc, Mutex};

/// Vulkan memory manager using gpu-allocator.
pub struct VkMemoryManager {
    /// The gpu-allocator instance.
    allocator: Arc<Mutex<Allocator>>,
    /// Device reference.
    device: Arc<VkDevice>,
}

impl VkMemoryManager {
    /// Creates a new memory manager.
    pub fn new(
        device: Arc<VkDevice>,
        instance: &ash::Instance,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.device.clone(),
            physical_device: device.physical_device,
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })?;

        Ok(Self {
            allocator: Arc::new(Mutex::new(allocator)),
            device,
        })
    }

    /// Allocates memory for a buffer.
    pub fn allocate_buffer_memory(
        &self,
        buffer: vk::Buffer,
        location: MemoryLocation,
        name: &str,
    ) -> Result<VkAllocation, Box<dyn std::error::Error>> {
        let requirements = unsafe { self.device.device.get_buffer_memory_requirements(buffer) };

        let allocation = self
            .allocator
            .lock()
            .unwrap()
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location,
                linear: true,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })?;

        unsafe {
            self.device.device.bind_buffer_memory(
                buffer,
                allocation.memory(),
                allocation.offset(),
            )?;
        }

        Ok(VkAllocation {
            allocation: Some(allocation),
            allocator: self.allocator.clone(),
        })
    }

    /// Allocates memory for an image.
    pub fn allocate_image_memory(
        &self,
        image: vk::Image,
        location: MemoryLocation,
        name: &str,
    ) -> Result<VkAllocation, Box<dyn std::error::Error>> {
        let requirements = unsafe { self.device.device.get_image_memory_requirements(image) };

        let allocation = self
            .allocator
            .lock()
            .unwrap()
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location,
                linear: false,
                allocation_scheme: gpu_allocator::vulkan::AllocationScheme::GpuAllocatorManaged,
            })?;

        unsafe {
            self.device.device.bind_image_memory(
                image,
                allocation.memory(),
                allocation.offset(),
            )?;
        }

        Ok(VkAllocation {
            allocation: Some(allocation),
            allocator: self.allocator.clone(),
        })
    }

    /// Creates a buffer with allocated memory.
    pub fn create_buffer(
        &self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        location: MemoryLocation,
        name: &str,
    ) -> Result<VkBuffer, Box<dyn std::error::Error>> {
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.device.device.create_buffer(&buffer_info, None)? };

        let allocation = self.allocate_buffer_memory(buffer, location, name)?;

        Ok(VkBuffer {
            buffer,
            allocation,
            size,
            device: self.device.clone(),
        })
    }

    /// Creates an image with allocated memory.
    pub fn create_image(
        &self,
        image_type: vk::ImageType,
        format: vk::Format,
        extent: vk::Extent3D,
        mip_levels: u32,
        array_layers: u32,
        samples: vk::SampleCountFlags,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        location: MemoryLocation,
        name: &str,
    ) -> Result<VkImage, Box<dyn std::error::Error>> {
        let image_info = vk::ImageCreateInfo::default()
            .image_type(image_type)
            .extent(extent)
            .mip_levels(mip_levels)
            .array_layers(array_layers)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(samples);

        let image = unsafe { self.device.device.create_image(&image_info, None)? };

        let allocation = self.allocate_image_memory(image, location, name)?;

        Ok(VkImage {
            image,
            allocation,
            format,
            extent,
            mip_levels,
            array_layers,
            device: self.device.clone(),
        })
    }

    /// Gets the device reference.
    pub fn device(&self) -> &Arc<VkDevice> {
        &self.device
    }
}

/// A wrapper for gpu-allocator allocation.
pub struct VkAllocation {
    /// The allocation (None after being moved out).
    allocation: Option<gpu_allocator::vulkan::Allocation>,
    /// Reference to the allocator for cleanup.
    allocator: Arc<Mutex<Allocator>>,
}

impl VkAllocation {
    /// Gets the mapped pointer if the allocation is host-visible.
    pub fn mapped_ptr(&self) -> Option<*mut u8> {
        self.allocation
            .as_ref()?
            .mapped_ptr()
            .map(|ptr| ptr.as_ptr() as *mut u8)
    }

    /// Gets the device memory handle.
    pub fn memory(&self) -> vk::DeviceMemory {
        unsafe { self.allocation.as_ref().unwrap().memory() }
    }

    /// Gets the offset within the memory.
    pub fn offset(&self) -> vk::DeviceSize {
        self.allocation.as_ref().unwrap().offset()
    }

    /// Gets the size of the allocation.
    pub fn size(&self) -> vk::DeviceSize {
        self.allocation.as_ref().unwrap().size()
    }
}

impl Drop for VkAllocation {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation.take() {
            if let Ok(mut allocator) = self.allocator.try_lock() {
                let _ = allocator.free(allocation);
            }
        }
    }
}

/// A Vulkan buffer with its allocation.
pub struct VkBuffer {
    /// The buffer handle.
    pub buffer: vk::Buffer,
    /// The memory allocation.
    pub allocation: VkAllocation,
    /// Buffer size.
    pub size: vk::DeviceSize,
    /// Device reference.
    device: Arc<VkDevice>,
}

impl VkBuffer {
    /// Maps the buffer memory for CPU access.
    pub fn map(&self) -> Option<*mut u8> {
        self.allocation.mapped_ptr()
    }

    /// Copies data to the buffer (assumes it's host-visible).
    pub fn copy_from_slice<T: Copy>(&self, data: &[T]) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mapped_ptr) = self.map() {
            let data_size = std::mem::size_of_val(data);
            if data_size > self.size as usize {
                return Err("Data size exceeds buffer size".into());
            }

            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr() as *const u8, mapped_ptr, data_size);
            }
            Ok(())
        } else {
            Err("Buffer is not host-visible".into())
        }
    }

    /// Copies data from the buffer (assumes it's host-visible).
    pub fn copy_to_slice<T: Copy>(&self, data: &mut [T]) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mapped_ptr) = self.map() {
            let data_size = std::mem::size_of_val(data);
            if data_size > self.size as usize {
                return Err("Data size exceeds buffer size".into());
            }

            unsafe {
                std::ptr::copy_nonoverlapping(mapped_ptr, data.as_mut_ptr() as *mut u8, data_size);
            }
            Ok(())
        } else {
            Err("Buffer is not host-visible".into())
        }
    }
}

impl Drop for VkBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_buffer(self.buffer, None);
        }
    }
}

/// A Vulkan image with its allocation.
pub struct VkImage {
    /// The image handle.
    pub image: vk::Image,
    /// The memory allocation.
    pub allocation: VkAllocation,
    /// Image format.
    pub format: vk::Format,
    /// Image extent.
    pub extent: vk::Extent3D,
    /// Number of mip levels.
    pub mip_levels: u32,
    /// Number of array layers.
    pub array_layers: u32,
    /// Device reference.
    device: Arc<VkDevice>,
}

impl VkImage {
    /// Creates an image view for this image.
    pub fn create_image_view(
        &self,
        view_type: vk::ImageViewType,
        aspect_mask: vk::ImageAspectFlags,
        base_mip_level: u32,
        level_count: u32,
        base_array_layer: u32,
        layer_count: u32,
    ) -> Result<vk::ImageView, vk::Result> {
        let create_info = vk::ImageViewCreateInfo::default()
            .image(self.image)
            .view_type(view_type)
            .format(self.format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level,
                level_count,
                base_array_layer,
                layer_count,
            });

        unsafe { self.device.device.create_image_view(&create_info, None) }
    }

    /// Transitions the image layout using a pipeline barrier.
    pub fn transition_layout(
        &self,
        command_buffer: vk::CommandBuffer,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        aspect_mask: vk::ImageAspectFlags,
    ) {
        let (src_access_mask, dst_access_mask) = match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => {
                (vk::AccessFlags::empty(), vk::AccessFlags::TRANSFER_WRITE)
            }
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
            ),
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            ),
            _ => (vk::AccessFlags::empty(), vk::AccessFlags::empty()),
        };

        let barrier = vk::ImageMemoryBarrier::default()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: self.mip_levels,
                base_array_layer: 0,
                layer_count: self.array_layers,
            })
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask);

        unsafe {
            self.device.device.cmd_pipeline_barrier(
                command_buffer,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    }
}

impl Drop for VkImage {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_image(self.image, None);
        }
    }
}
