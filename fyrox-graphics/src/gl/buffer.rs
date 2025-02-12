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

use crate::gl::server::GlGraphicsServer;
use crate::gl::ToGlConstant;
use crate::{
    buffer::{BufferKind, BufferUsage, GpuBufferTrait},
    error::FrameworkError,
};
use glow::HasContext;
use std::{cell::Cell, rc::Weak};

impl ToGlConstant for BufferKind {
    fn into_gl(self) -> u32 {
        match self {
            BufferKind::Vertex => glow::ARRAY_BUFFER,
            BufferKind::Index => glow::ELEMENT_ARRAY_BUFFER,
            BufferKind::Uniform => glow::UNIFORM_BUFFER,
            BufferKind::PixelRead => glow::PIXEL_PACK_BUFFER,
            BufferKind::PixelWrite => glow::PIXEL_UNPACK_BUFFER,
        }
    }
}

impl ToGlConstant for BufferUsage {
    fn into_gl(self) -> u32 {
        match self {
            BufferUsage::StaticDraw => glow::STATIC_DRAW,
            BufferUsage::StaticCopy => glow::STATIC_COPY,
            BufferUsage::DynamicDraw => glow::DYNAMIC_DRAW,
            BufferUsage::DynamicCopy => glow::DYNAMIC_COPY,
            BufferUsage::StreamDraw => glow::STREAM_DRAW,
            BufferUsage::StreamRead => glow::STREAM_READ,
            BufferUsage::StreamCopy => glow::STREAM_COPY,
            BufferUsage::StaticRead => glow::STATIC_READ,
            BufferUsage::DynamicRead => glow::DYNAMIC_READ,
        }
    }
}

pub struct GlBuffer {
    pub state: Weak<GlGraphicsServer>,
    pub id: glow::Buffer,
    pub size: Cell<usize>,
    pub kind: BufferKind,
    pub usage: BufferUsage,
}

impl GlBuffer {
    pub fn new(
        server: &GlGraphicsServer,
        size_bytes: usize,
        kind: BufferKind,
        usage: BufferUsage,
    ) -> Result<Self, FrameworkError> {
        unsafe {
            let gl_kind = kind.into_gl();
            let gl_usage = usage.into_gl();
            let id = server.gl.create_buffer()?;
            server.gl.bind_buffer(gl_kind, Some(id));
            if size_bytes > 0 {
                server
                    .gl
                    .buffer_data_size(gl_kind, size_bytes as i32, gl_usage);
            }
            server.gl.bind_buffer(gl_kind, None);
            Ok(Self {
                state: server.weak(),
                id,
                size: Cell::new(size_bytes),
                kind,
                usage,
            })
        }
    }
}

impl Drop for GlBuffer {
    fn drop(&mut self) {
        unsafe {
            if let Some(state) = self.state.upgrade() {
                state.gl.delete_buffer(self.id);
            }
        }
    }
}

impl GpuBufferTrait for GlBuffer {
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

        let Some(server) = self.state.upgrade() else {
            return Err(FrameworkError::GraphicsServerUnavailable);
        };

        let gl_kind = self.kind.into_gl();
        let gl_usage = self.usage.into_gl();

        unsafe {
            server.gl.bind_buffer(gl_kind, Some(self.id));
            if data.len() <= self.size.get() {
                // Update the data.
                server.gl.buffer_sub_data_u8_slice(gl_kind, 0, data);
            } else {
                // Realloc the internal storage.
                server.gl.buffer_data_u8_slice(gl_kind, data, gl_usage);
                self.size.set(data.len());
            }
        }

        Ok(())
    }

    fn read_data(&self, data: &mut [u8]) -> Result<(), FrameworkError> {
        let Some(server) = self.state.upgrade() else {
            return Err(FrameworkError::GraphicsServerUnavailable);
        };

        let gl_kind = self.kind.into_gl();

        unsafe {
            server.gl.bind_buffer(gl_kind, Some(self.id));

            #[cfg(not(target_arch = "wasm32"))]
            {
                let gl_storage =
                    server
                        .gl
                        .map_buffer_range(gl_kind, 0, data.len() as i32, glow::MAP_READ_BIT);
                assert_ne!(gl_storage, std::ptr::null_mut());
                std::ptr::copy_nonoverlapping(gl_storage, data.as_mut_ptr(), data.len());
                server.gl.unmap_buffer(gl_kind);
            }

            #[cfg(target_arch = "wasm32")]
            {
                // The only way to get buffer data on WebGL is to use glGetBufferSubData, there's
                // no memory mapping in Web due to security reasons.
                server.gl.get_buffer_sub_data(gl_kind, 0, data);
            }

            server.gl.bind_buffer(gl_kind, None);
        }

        Ok(())
    }
}
