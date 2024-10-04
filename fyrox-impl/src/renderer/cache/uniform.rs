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
    buffer::{Buffer, BufferKind, BufferUsage},
    error::FrameworkError,
    server::GraphicsServer,
};
use fxhash::FxHashMap;
use fyrox_graphics::uniform::{ByteStorage, UniformBuffer};
use std::cell::RefCell;

#[derive(Default)]
struct UniformBufferSet {
    buffers: Vec<Box<dyn Buffer>>,
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
    ) -> Result<&dyn Buffer, FrameworkError> {
        if self.free < self.buffers.len() {
            let buffer = &self.buffers[self.free];
            self.free += 1;
            Ok(&**buffer)
        } else {
            let buffer =
                server.create_buffer(size, BufferKind::Uniform, BufferUsage::StreamCopy)?;
            self.buffers.push(buffer);
            self.free = self.buffers.len();
            Ok(&**self.buffers.last().unwrap())
        }
    }
}

/// Uniform buffer cache could be considered as pool of uniform buffers of fixed size, that can be
/// used to fetch free buffer for drawing. Uniform buffers usually have quite limited size
/// (guaranteed to be at least 16kb and on vast majority of GPUs the upper limit is 65kb) and they
/// are intended to be used as a storage for relatively small set of data that can fit into L1 cache
/// of a GPU for very fast access.
#[derive(Default)]
pub struct UniformBufferCache {
    cache: RefCell<FxHashMap<usize, UniformBufferSet>>,
}

impl UniformBufferCache {
    /// Reserves one of the existing uniform buffers of the given size. If there's no such free buffer,
    /// this method creates a new one and reserves it for further use.
    pub fn get_or_create<'a>(
        &'a self,
        server: &dyn GraphicsServer,
        size: usize,
    ) -> Result<&'a dyn Buffer, FrameworkError> {
        let mut cache = self.cache.borrow_mut();
        let set = cache.entry(size).or_default();
        let buffer = set.get_or_create(size, server)?;
        // SAFETY: GPU buffer "lives" in memory heap, so any potential memory reallocation of the
        // hash map won't affect the returned reference. Also, buffers cannot be deleted so the
        // reference is also valid. These reasons allows to extend lifetime to the lifetime of self.
        Ok(unsafe { std::mem::transmute::<&'_ dyn Buffer, &'a dyn Buffer>(buffer) })
    }

    /// Fetches a suitable (or creates new one) GPU uniform buffer for the given CPU uniform buffer
    /// and writes the data to it, returns a reference to the buffer.
    pub fn write<T>(
        &self,
        server: &dyn GraphicsServer,
        uniform_buffer: UniformBuffer<T>,
    ) -> Result<&dyn Buffer, FrameworkError>
    where
        T: ByteStorage,
    {
        let data = uniform_buffer.finish();
        let buffer = self.get_or_create(server, data.bytes_count())?;
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
