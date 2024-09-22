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

use crate::uniform_buffer::UniformBuffer;
use crate::{error::FrameworkError, state::GlGraphicsServer};
use glow::HasContext;
use std::rc::Weak;

pub struct GlUniformBuffer {
    state: Weak<GlGraphicsServer>,
    id: glow::Buffer,
    size: usize,
}

impl GlUniformBuffer {
    pub fn new(server: &GlGraphicsServer, size_bytes: usize) -> Result<Self, FrameworkError> {
        unsafe {
            let id = server.gl.create_buffer()?;
            server.gl.bind_buffer(glow::UNIFORM_BUFFER, Some(id));
            server
                .gl
                .buffer_data_size(glow::UNIFORM_BUFFER, size_bytes as i32, glow::DYNAMIC_COPY);
            server.gl.bind_buffer(glow::UNIFORM_BUFFER, None);
            Ok(Self {
                state: server.weak(),
                id,
                size: size_bytes,
            })
        }
    }
}

impl Drop for GlUniformBuffer {
    fn drop(&mut self) {
        unsafe {
            if let Some(state) = self.state.upgrade() {
                state.gl.delete_buffer(self.id);
            }
        }
    }
}

impl UniformBuffer for GlUniformBuffer {
    fn write_data(&self, data: &[u8]) -> Result<(), FrameworkError> {
        if data.len() != self.size {
            return Err(FrameworkError::Custom(format!(
                "Uniform buffer size {} does not match the data size {}",
                self.size,
                data.len()
            )));
        }

        if let Some(state) = self.state.upgrade() {
            unsafe {
                state.gl.bind_buffer(glow::UNIFORM_BUFFER, Some(self.id));
                state
                    .gl
                    .buffer_data_u8_slice(glow::UNIFORM_BUFFER, data, glow::DYNAMIC_COPY);
                state.gl.bind_buffer(glow::UNIFORM_BUFFER, None);
            }
        }

        Ok(())
    }
}
