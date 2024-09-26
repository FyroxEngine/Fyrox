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
    state::GraphicsServer,
};
use fxhash::FxHashMap;

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
    cache: FxHashMap<usize, UniformBufferSet>,
}

impl UniformBufferCache {
    /// Reserves one of the existing uniform buffers of the given size. If there's no such free buffer,
    /// this method creates a new one and reserves it for further use.
    pub fn get_or_create(
        &mut self,
        server: &dyn GraphicsServer,
        size: usize,
    ) -> Result<&dyn Buffer, FrameworkError> {
        let set = self.cache.entry(size).or_default();
        set.get_or_create(size, server)
    }

    /// Marks all reserved buffers as unused. Must be called at least once per frame to prevent
    /// uncontrollable growth of the cache.
    pub fn mark_all_unused(&mut self) {
        for set in self.cache.values_mut() {
            set.mark_unused();
        }
    }
}
