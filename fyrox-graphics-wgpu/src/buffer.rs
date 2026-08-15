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

use crate::server::WgpuGraphicsServer;
use fyrox_graphics::{
    buffer::{BufferKind, BufferUsage, GpuBufferDescriptor, GpuBufferTrait},
    error::FrameworkError,
};
use std::cell::{Cell, RefCell};
use std::rc::Weak;

/// Maps a Fyrox [`BufferKind`] to the corresponding wgpu [`BufferUsages`] flags.
///
/// All buffers additionally get `COPY_DST` to allow data uploads via `queue.write_buffer`.
/// `PixelWrite` buffers also get `COPY_SRC` to allow copying to readback buffers.
fn buffer_usage_to_wgpu(kind: BufferKind) -> wgpu::BufferUsages {
    let mut flags = match kind {
        BufferKind::Vertex => wgpu::BufferUsages::VERTEX,
        BufferKind::Index => wgpu::BufferUsages::INDEX,
        BufferKind::Uniform => wgpu::BufferUsages::UNIFORM,
        BufferKind::PixelRead => wgpu::BufferUsages::MAP_READ,
        BufferKind::PixelWrite => wgpu::BufferUsages::MAP_WRITE,
    };
    flags |= wgpu::BufferUsages::COPY_DST;
    if kind == BufferKind::PixelWrite {
        flags |= wgpu::BufferUsages::COPY_SRC;
    }
    flags
}

/// Wgpu implementation of [`GpuBufferTrait`](fyrox_graphics::buffer::GpuBufferTrait).
///
/// Wraps a [`wgpu::Buffer`] and tracks its memory usage via [`ServerMemoryUsage`].
/// When `write_data` receives data larger than the current buffer capacity, the buffer
/// is transparently reallocated to fit — matching the OpenGL backend behavior. Memory
/// accounting is updated accordingly.
pub struct WgpuBuffer {
    server: Weak<WgpuGraphicsServer>,
    buffer: RefCell<wgpu::Buffer>,
    size: Cell<usize>,
    kind: BufferKind,
    usage: BufferUsage,
}

impl WgpuBuffer {
    /// Creates a new GPU buffer from the given descriptor.
    ///
    /// The buffer is created with at least 1 byte of capacity (wgpu requires non-zero size).
    /// Memory usage is tracked on the server.
    pub fn new(
        server: &WgpuGraphicsServer,
        desc: GpuBufferDescriptor,
    ) -> Result<Self, FrameworkError> {
        let wgpu_usage = buffer_usage_to_wgpu(desc.kind);
        let buffer = server.state.device.create_buffer(&wgpu::BufferDescriptor {
            label: if server.named_objects {
                Some(desc.name)
            } else {
                None
            },
            size: desc.size.max(1) as u64,
            usage: wgpu_usage,
            mapped_at_creation: false,
        });
        server.memory_usage.borrow_mut().buffers += desc.size;
        Ok(Self {
            server: server.weak_ref(),
            buffer: RefCell::new(buffer),
            size: Cell::new(desc.size),
            kind: desc.kind,
            usage: desc.usage,
        })
    }

    /// Returns a reference to the underlying [`wgpu::Buffer`].
    ///
    /// # Safety
    ///
    /// The returned reference must not be held across a call to `write_data`,
    /// which may replace the inner buffer. In practice this is always satisfied
    /// because `write_data` takes `&self` and completes before external access.
    pub unsafe fn wgpu_buffer_raw(&self) -> &wgpu::Buffer {
        // SAFETY: The caller guarantees no mutable borrow is active.
        unsafe { &*self.buffer.as_ptr() }
    }
}

impl Drop for WgpuBuffer {
    fn drop(&mut self) {
        if let Some(server) = self.server.upgrade() {
            server.memory_usage.borrow_mut().buffers -= self.size.get();
        }
    }
}

impl GpuBufferTrait for WgpuBuffer {
    fn usage(&self) -> BufferUsage {
        self.usage
    }
    fn kind(&self) -> BufferKind {
        self.kind
    }
    fn size(&self) -> usize {
        self.size.get()
    }

    fn write_data(&self, data: &[u8]) -> Result<(), FrameworkError> {
        if data.is_empty() {
            return Ok(());
        }
        let Some(server) = self.server.upgrade() else {
            return Err(FrameworkError::GraphicsServerUnavailable);
        };
        if data.len() <= self.size.get() {
            server
                .state
                .queue
                .write_buffer(&self.buffer.borrow(), 0, data);
        } else {
            // Reallocate the buffer to fit the larger data, matching the GL backend
            // behavior. This prevents silent data truncation that caused rendering
            // artifacts (e.g. broken skeletal animation when bone count changes).
            let new_size = data.len();
            let wgpu_usage = buffer_usage_to_wgpu(self.kind);
            let new_buffer = server.state.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: new_size as u64,
                usage: wgpu_usage,
                mapped_at_creation: false,
            });
            server.state.queue.write_buffer(&new_buffer, 0, data);

            let mut mem = server.memory_usage.borrow_mut();
            mem.buffers -= self.size.get();
            mem.buffers += new_size;

            *self.buffer.borrow_mut() = new_buffer;
            self.size.set(new_size);
        }
        Ok(())
    }

    fn read_data(&self, data: &mut [u8]) -> Result<(), FrameworkError> {
        let Some(server) = self.server.upgrade() else {
            return Err(FrameworkError::GraphicsServerUnavailable);
        };

        let buf = self.buffer.borrow();
        let buffer_slice = buf.slice(..data.len() as u64);
        let (tx, rx) = std::sync::mpsc::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).ok();
        });

        server
            .state
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .ok();

        rx.recv()
            .map_err(|_| FrameworkError::Custom("Channel closed".into()))?
            .map_err(|e| FrameworkError::Custom(format!("Buffer map failed: {e}")))?;

        let mapped = buffer_slice
            .get_mapped_range()
            .map_err(|e| FrameworkError::Custom(format!("Failed to get mapped range: {e}")))?;

        data.copy_from_slice(&mapped);
        drop(mapped);
        buf.unmap();

        Ok(())
    }
}
