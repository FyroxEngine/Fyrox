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
use std::sync::{Arc, Mutex};

/// Command buffer pool for managing command buffer allocation.
pub struct CommandPool {
    /// The command pool handle.
    pub pool: vk::CommandPool,
    /// The device reference.
    device: Arc<VkDevice>,
    /// Queue family index this pool is associated with.
    #[allow(dead_code)]
    queue_family_index: u32,
}

impl CommandPool {
    /// Creates a new command pool.
    pub fn new(
        device: Arc<VkDevice>,
        queue_family_index: u32,
        flags: vk::CommandPoolCreateFlags,
    ) -> Result<Self, vk::Result> {
        let create_info = vk::CommandPoolCreateInfo::default()
            .flags(flags)
            .queue_family_index(queue_family_index);

        let pool = unsafe { device.device.create_command_pool(&create_info, None)? };

        Ok(Self {
            pool,
            device,
            queue_family_index,
        })
    }

    /// Allocates a single command buffer.
    pub fn allocate_command_buffer(
        &self,
        level: vk::CommandBufferLevel,
    ) -> Result<vk::CommandBuffer, vk::Result> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(level)
            .command_buffer_count(1);

        let command_buffers = unsafe { self.device.device.allocate_command_buffers(&alloc_info)? };

        Ok(command_buffers[0])
    }

    /// Allocates multiple command buffers.
    pub fn allocate_command_buffers(
        &self,
        level: vk::CommandBufferLevel,
        count: u32,
    ) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(level)
            .command_buffer_count(count);

        unsafe { self.device.device.allocate_command_buffers(&alloc_info) }
    }

    /// Frees command buffers.
    pub fn free_command_buffers(&self, command_buffers: &[vk::CommandBuffer]) {
        unsafe {
            self.device
                .device
                .free_command_buffers(self.pool, command_buffers);
        }
    }

    /// Resets the command pool.
    pub fn reset(&self, flags: vk::CommandPoolResetFlags) -> Result<(), vk::Result> {
        unsafe { self.device.device.reset_command_pool(self.pool, flags) }
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_command_pool(self.pool, None);
        }
    }
}

/// A wrapper for a single command buffer with convenience methods.
pub struct CommandBuffer {
    /// The command buffer handle.
    pub buffer: vk::CommandBuffer,
    /// The device reference.
    device: Arc<VkDevice>,
    /// The command pool this buffer was allocated from.
    pool: Arc<Mutex<CommandPool>>,
}

impl CommandBuffer {
    /// Creates a new command buffer from a pool.
    pub fn new(
        device: Arc<VkDevice>,
        pool: Arc<Mutex<CommandPool>>,
        level: vk::CommandBufferLevel,
    ) -> Result<Self, vk::Result> {
        let buffer = pool.lock().unwrap().allocate_command_buffer(level)?;

        Ok(Self {
            buffer,
            device,
            pool,
        })
    }

    /// Begins recording commands.
    pub fn begin(&self, flags: vk::CommandBufferUsageFlags) -> Result<(), vk::Result> {
        let begin_info = vk::CommandBufferBeginInfo::default().flags(flags);

        unsafe {
            self.device
                .device
                .begin_command_buffer(self.buffer, &begin_info)
        }
    }

    /// Ends recording commands.
    pub fn end(&self) -> Result<(), vk::Result> {
        unsafe { self.device.device.end_command_buffer(self.buffer) }
    }

    /// Resets the command buffer.
    pub fn reset(&self, flags: vk::CommandBufferResetFlags) -> Result<(), vk::Result> {
        unsafe { self.device.device.reset_command_buffer(self.buffer, flags) }
    }

    /// Begins a render pass.
    pub fn begin_render_pass(
        &self,
        render_pass_begin: &vk::RenderPassBeginInfo,
        contents: vk::SubpassContents,
    ) {
        unsafe {
            self.device
                .device
                .cmd_begin_render_pass(self.buffer, render_pass_begin, contents);
        }
    }

    /// Ends the current render pass.
    pub fn end_render_pass(&self) {
        unsafe {
            self.device.device.cmd_end_render_pass(self.buffer);
        }
    }

    /// Binds a graphics pipeline.
    pub fn bind_pipeline(
        &self,
        pipeline_bind_point: vk::PipelineBindPoint,
        pipeline: vk::Pipeline,
    ) {
        unsafe {
            self.device
                .device
                .cmd_bind_pipeline(self.buffer, pipeline_bind_point, pipeline);
        }
    }

    /// Binds vertex buffers.
    pub fn bind_vertex_buffers(
        &self,
        first_binding: u32,
        buffers: &[vk::Buffer],
        offsets: &[vk::DeviceSize],
    ) {
        unsafe {
            self.device.device.cmd_bind_vertex_buffers(
                self.buffer,
                first_binding,
                buffers,
                offsets,
            );
        }
    }

    /// Binds an index buffer.
    pub fn bind_index_buffer(
        &self,
        buffer: vk::Buffer,
        offset: vk::DeviceSize,
        index_type: vk::IndexType,
    ) {
        unsafe {
            self.device
                .device
                .cmd_bind_index_buffer(self.buffer, buffer, offset, index_type);
        }
    }

    /// Binds descriptor sets.
    pub fn bind_descriptor_sets(
        &self,
        pipeline_bind_point: vk::PipelineBindPoint,
        layout: vk::PipelineLayout,
        first_set: u32,
        descriptor_sets: &[vk::DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        unsafe {
            self.device.device.cmd_bind_descriptor_sets(
                self.buffer,
                pipeline_bind_point,
                layout,
                first_set,
                descriptor_sets,
                dynamic_offsets,
            );
        }
    }

    /// Issues a draw command.
    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.device.cmd_draw(
                self.buffer,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            );
        }
    }

    /// Issues an indexed draw command.
    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.device.device.cmd_draw_indexed(
                self.buffer,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            );
        }
    }

    /// Copies from one buffer to another.
    pub fn copy_buffer(
        &self,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        regions: &[vk::BufferCopy],
    ) {
        unsafe {
            self.device
                .device
                .cmd_copy_buffer(self.buffer, src_buffer, dst_buffer, regions);
        }
    }

    /// Copies from buffer to image.
    pub fn copy_buffer_to_image(
        &self,
        buffer: vk::Buffer,
        image: vk::Image,
        layout: vk::ImageLayout,
        regions: &[vk::BufferImageCopy],
    ) {
        unsafe {
            self.device.device.cmd_copy_buffer_to_image(
                self.buffer,
                buffer,
                image,
                layout,
                regions,
            );
        }
    }

    /// Copies from image to buffer.
    pub fn copy_image_to_buffer(
        &self,
        image: vk::Image,
        layout: vk::ImageLayout,
        buffer: vk::Buffer,
        regions: &[vk::BufferImageCopy],
    ) {
        unsafe {
            self.device.device.cmd_copy_image_to_buffer(
                self.buffer,
                image,
                layout,
                buffer,
                regions,
            );
        }
    }

    /// Transitions image layout using a pipeline barrier.
    pub fn pipeline_barrier(
        &self,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        dependency_flags: vk::DependencyFlags,
        memory_barriers: &[vk::MemoryBarrier],
        buffer_memory_barriers: &[vk::BufferMemoryBarrier],
        image_memory_barriers: &[vk::ImageMemoryBarrier],
    ) {
        unsafe {
            self.device.device.cmd_pipeline_barrier(
                self.buffer,
                src_stage_mask,
                dst_stage_mask,
                dependency_flags,
                memory_barriers,
                buffer_memory_barriers,
                image_memory_barriers,
            );
        }
    }

    /// Sets viewport.
    pub fn set_viewport(&self, first_viewport: u32, viewports: &[vk::Viewport]) {
        unsafe {
            self.device
                .device
                .cmd_set_viewport(self.buffer, first_viewport, viewports);
        }
    }

    /// Sets scissor rectangles.
    pub fn set_scissor(&self, first_scissor: u32, scissors: &[vk::Rect2D]) {
        unsafe {
            self.device
                .device
                .cmd_set_scissor(self.buffer, first_scissor, scissors);
        }
    }

    /// Pushes constants to the pipeline.
    pub fn push_constants(
        &self,
        layout: vk::PipelineLayout,
        stage_flags: vk::ShaderStageFlags,
        offset: u32,
        constants: &[u8],
    ) {
        unsafe {
            self.device.device.cmd_push_constants(
                self.buffer,
                layout,
                stage_flags,
                offset,
                constants,
            );
        }
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        let pool = self.pool.lock().unwrap();
        pool.free_command_buffers(&[self.buffer]);
    }
}

/// A command manager that handles multiple command pools and command buffer allocation.
pub struct CommandManager {
    /// Graphics command pool.
    graphics_pool: Arc<Mutex<CommandPool>>,
    /// Transfer command pool.
    transfer_pool: Arc<Mutex<CommandPool>>,
    /// Compute command pool.
    compute_pool: Arc<Mutex<CommandPool>>,
    /// Device reference.
    device: Arc<VkDevice>,
}

impl CommandManager {
    /// Creates a new command manager.
    pub fn new(device: Arc<VkDevice>) -> Result<Self, vk::Result> {
        let graphics_pool = Arc::new(Mutex::new(CommandPool::new(
            device.clone(),
            device.queue_families.graphics_family.unwrap(),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        )?));

        let transfer_pool = Arc::new(Mutex::new(CommandPool::new(
            device.clone(),
            device
                .queue_families
                .transfer_family
                .unwrap_or(device.queue_families.graphics_family.unwrap()),
            vk::CommandPoolCreateFlags::TRANSIENT,
        )?));

        let compute_pool = Arc::new(Mutex::new(CommandPool::new(
            device.clone(),
            device
                .queue_families
                .compute_family
                .unwrap_or(device.queue_families.graphics_family.unwrap()),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        )?));

        Ok(Self {
            graphics_pool,
            transfer_pool,
            compute_pool,
            device,
        })
    }

    /// Allocates a graphics command buffer.
    pub fn allocate_graphics_command_buffer(&self) -> Result<CommandBuffer, vk::Result> {
        CommandBuffer::new(
            self.device.clone(),
            self.graphics_pool.clone(),
            vk::CommandBufferLevel::PRIMARY,
        )
    }

    /// Allocates a transfer command buffer.
    pub fn allocate_transfer_command_buffer(&self) -> Result<CommandBuffer, vk::Result> {
        CommandBuffer::new(
            self.device.clone(),
            self.transfer_pool.clone(),
            vk::CommandBufferLevel::PRIMARY,
        )
    }

    /// Allocates a compute command buffer.
    pub fn allocate_compute_command_buffer(&self) -> Result<CommandBuffer, vk::Result> {
        CommandBuffer::new(
            self.device.clone(),
            self.compute_pool.clone(),
            vk::CommandBufferLevel::PRIMARY,
        )
    }

    /// Executes a one-time submit command buffer on the graphics queue.
    pub fn execute_one_time_graphics<F>(&self, f: F) -> Result<(), vk::Result>
    where
        F: FnOnce(&CommandBuffer) -> Result<(), vk::Result>,
    {
        let command_buffer = self.allocate_graphics_command_buffer()?;

        command_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)?;
        f(&command_buffer)?;
        command_buffer.end()?;

        let buffers = [command_buffer.buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&buffers);

        unsafe {
            self.device.device.queue_submit(
                self.device.graphics_queue,
                &[submit_info],
                vk::Fence::null(),
            )?;
            self.device
                .device
                .queue_wait_idle(self.device.graphics_queue)?;
        }

        Ok(())
    }

    /// Executes a one-time submit command buffer on the transfer queue.
    pub fn execute_one_time_transfer<F>(&self, f: F) -> Result<(), vk::Result>
    where
        F: FnOnce(&CommandBuffer) -> Result<(), vk::Result>,
    {
        let command_buffer = self.allocate_transfer_command_buffer()?;

        command_buffer.begin(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)?;
        f(&command_buffer)?;
        command_buffer.end()?;

        let buffers = [command_buffer.buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&buffers);

        unsafe {
            self.device.device.queue_submit(
                self.device.transfer_queue,
                &[submit_info],
                vk::Fence::null(),
            )?;
            self.device
                .device
                .queue_wait_idle(self.device.transfer_queue)?;
        }

        Ok(())
    }
}
