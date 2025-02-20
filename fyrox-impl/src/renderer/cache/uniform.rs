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

//! Uniform buffer cache could be considered as pool of uniform buffers of fixed size. See
//! [`UniformBufferCache`] for more info.

use crate::renderer::framework::{
    buffer::{BufferKind, BufferUsage},
    error::FrameworkError,
    framebuffer::{BufferDataUsage, ResourceBinding},
    server::GraphicsServer,
    uniform::{ByteStorage, DynamicUniformBuffer, UniformBuffer},
};
use fxhash::FxHashMap;
use fyrox_graphics::buffer::GpuBuffer;
use fyrox_graphics::server::SharedGraphicsServer;
use std::cell::RefCell;

#[derive(Default)]
struct UniformBufferSet {
    buffers: Vec<GpuBuffer>,
    free: usize,
}

impl UniformBufferSet {
    fn mark_unused(&mut self) {
        self.free = 0;
    }

    fn get_or_create(
        &mut self,
        size: usize,
        server: &dyn GraphicsServer,
    ) -> Result<GpuBuffer, FrameworkError> {
        if self.free < self.buffers.len() {
            let buffer = &self.buffers[self.free];
            self.free += 1;
            Ok(buffer.clone())
        } else {
            let buffer =
                server.create_buffer(size, BufferKind::Uniform, BufferUsage::StreamCopy)?;
            self.buffers.push(buffer);
            self.free = self.buffers.len();
            Ok(self.buffers.last().unwrap().clone())
        }
    }
}

/// Uniform buffer cache could be considered as pool of uniform buffers of fixed size, that can be
/// used to fetch free buffer for drawing. Uniform buffers usually have quite limited size
/// (guaranteed to be at least 16kb and on vast majority of GPUs the upper limit is 65kb) and they
/// are intended to be used as a storage for relatively small set of data that can fit into L1 cache
/// of a GPU for very fast access.
pub struct UniformBufferCache {
    server: SharedGraphicsServer,
    cache: RefCell<FxHashMap<usize, UniformBufferSet>>,
}

impl UniformBufferCache {
    pub fn new(server: SharedGraphicsServer) -> Self {
        Self {
            server,
            cache: Default::default(),
        }
    }

    /// Reserves one of the existing uniform buffers of the given size. If there's no such free buffer,
    /// this method creates a new one and reserves it for further use.
    pub fn get_or_create(&self, size: usize) -> Result<GpuBuffer, FrameworkError> {
        let mut cache = self.cache.borrow_mut();
        let set = cache.entry(size).or_default();
        set.get_or_create(size, &*self.server)
    }

    /// Fetches a suitable (or creates new one) GPU uniform buffer for the given CPU uniform buffer
    /// and writes the data to it, returns a reference to the buffer.
    pub fn write<T>(&self, uniform_buffer: UniformBuffer<T>) -> Result<GpuBuffer, FrameworkError>
    where
        T: ByteStorage,
    {
        let data = uniform_buffer.finish();
        let buffer = self.get_or_create(data.bytes_count())?;
        buffer.write_data(data.bytes())?;
        Ok(buffer)
    }

    /// Marks all reserved buffers as unused. Must be called at least once per frame to prevent
    /// uncontrollable growth of the cache.
    pub fn mark_all_unused(&mut self) {
        for set in self.cache.borrow_mut().values_mut() {
            set.mark_unused();
        }
    }

    /// Returns the total amount of allocated uniforms buffers.
    pub fn alive_count(&self) -> usize {
        let mut count = 0;
        for (_, set) in self.cache.borrow().iter() {
            count += set.buffers.len();
        }
        count
    }
}

struct Page {
    dynamic: DynamicUniformBuffer,
    is_submitted: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct UniformBlockLocation {
    pub page: usize,
    pub offset: usize,
    pub size: usize,
}

pub struct UniformMemoryAllocator {
    gpu_buffers: Vec<GpuBuffer>,
    block_alignment: usize,
    max_uniform_buffer_size: usize,
    pages: Vec<Page>,
    blocks: Vec<UniformBlockLocation>,
}

impl UniformMemoryAllocator {
    pub fn new(max_uniform_buffer_size: usize, block_alignment: usize) -> Self {
        Self {
            gpu_buffers: Default::default(),
            block_alignment,
            max_uniform_buffer_size,
            pages: Default::default(),
            blocks: Default::default(),
        }
    }

    pub fn clear(&mut self) {
        for page in self.pages.iter_mut() {
            page.dynamic.clear();
            page.is_submitted = false;
        }
        self.blocks.clear();
    }

    pub fn allocate<T>(&mut self, buffer: UniformBuffer<T>) -> UniformBlockLocation
    where
        T: ByteStorage,
    {
        let data = buffer.finish();
        assert!(data.bytes_count() > 0);
        assert!(data.bytes_count() < self.max_uniform_buffer_size);

        let page_index = match self.pages.iter().position(|page| {
            let write_position = page
                .dynamic
                .next_write_aligned_position(self.block_alignment);
            self.max_uniform_buffer_size - write_position >= data.bytes_count()
        }) {
            Some(page_index) => page_index,
            None => {
                let page_index = self.pages.len();
                self.pages.push(Page {
                    dynamic: UniformBuffer::with_storage(Vec::with_capacity(
                        self.max_uniform_buffer_size,
                    )),
                    is_submitted: false,
                });
                page_index
            }
        };

        let page = &mut self.pages[page_index];
        page.is_submitted = false;
        let offset = page
            .dynamic
            .write_bytes_with_alignment(data.bytes(), self.block_alignment);

        let block = UniformBlockLocation {
            page: page_index,
            offset,
            size: data.bytes_count(),
        };
        self.blocks.push(block);
        block
    }

    pub fn upload(&mut self, server: &dyn GraphicsServer) -> Result<(), FrameworkError> {
        if self.gpu_buffers.len() < self.pages.len() {
            for _ in 0..(self.pages.len() - self.gpu_buffers.len()) {
                let buffer = server.create_buffer(
                    self.max_uniform_buffer_size,
                    BufferKind::Uniform,
                    BufferUsage::StreamCopy,
                )?;
                self.gpu_buffers.push(buffer);
            }
        }

        for (page, gpu_buffer) in self.pages.iter_mut().zip(self.gpu_buffers.iter()) {
            if !page.is_submitted {
                let bytes = page.dynamic.storage().bytes();
                assert!(bytes.len() <= self.max_uniform_buffer_size);
                gpu_buffer.write_data(bytes)?;
                page.is_submitted = true;
            }
        }

        Ok(())
    }

    pub fn block_to_binding(
        &self,
        block: UniformBlockLocation,
        binding_point: usize,
    ) -> ResourceBinding {
        ResourceBinding::Buffer {
            buffer: self.gpu_buffers[block.page].clone(),
            binding: binding_point,
            data_usage: BufferDataUsage::UseSegment {
                offset: block.offset,
                size: block.size,
            },
        }
    }
}
