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

use crate::{buffer::GlBuffer, server::GlGraphicsServer, ToGlConstant};
use fyrox_graphics::{
    buffer::{BufferKind, GpuBufferDescriptor, GpuBufferTrait},
    core::{array_as_u8_slice, math::TriangleDefinition},
    error::FrameworkError,
    geometry_buffer::{
        AttributeKind, ElementsDescriptor, GpuGeometryBufferDescriptor, GpuGeometryBufferTrait,
    },
    ElementKind,
};
use glow::HasContext;
use std::{cell::Cell, marker::PhantomData, rc::Weak};

impl ToGlConstant for AttributeKind {
    fn into_gl(self) -> u32 {
        match self {
            AttributeKind::Float => glow::FLOAT,
            AttributeKind::UnsignedByte => glow::UNSIGNED_BYTE,
            AttributeKind::UnsignedShort => glow::UNSIGNED_SHORT,
            AttributeKind::UnsignedInt => glow::UNSIGNED_INT,
        }
    }
}

pub struct GlGeometryBuffer {
    pub state: Weak<GlGraphicsServer>,
    pub vertex_array_object: glow::VertexArray,
    pub buffers: Vec<GlBuffer>,
    pub element_buffer: GlBuffer,
    pub element_count: Cell<usize>,
    pub element_kind: ElementKind,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

impl GlGeometryBuffer {
    pub fn new(
        server: &GlGraphicsServer,
        desc: GpuGeometryBufferDescriptor,
    ) -> Result<Self, FrameworkError> {
        let vao = unsafe { server.gl.create_vertex_array()? };

        server.set_vertex_array_object(Some(vao));
        #[cfg(not(target_arch = "wasm32"))]
        if server.gl.supports_debug() && server.named_objects.get() {
            unsafe {
                server
                    .gl
                    .object_label(glow::VERTEX_ARRAY, vao.0.get(), Some(desc.name));
            }
        }

        let element_buffer = GlBuffer::new(
            server,
            GpuBufferDescriptor {
                name: &format!("{}ElementBuffer", desc.name),
                size: 0,
                kind: BufferKind::Index,
                usage: desc.usage,
            },
        )?;

        let (element_count, data) = match desc.elements {
            ElementsDescriptor::Triangles(triangles) => {
                (triangles.len(), array_as_u8_slice(triangles))
            }
            ElementsDescriptor::Lines(lines) => (lines.len(), array_as_u8_slice(lines)),
            ElementsDescriptor::Points(points) => (points.len(), array_as_u8_slice(points)),
        };

        element_buffer.write_data(data)?;

        let mut buffers = Vec::new();
        for (i, buffer) in desc.buffers.iter().enumerate() {
            unsafe {
                let data_size = buffer.data.bytes.map(|bytes| bytes.len()).unwrap_or(0);

                let native_buffer = GlBuffer::new(
                    server,
                    GpuBufferDescriptor {
                        name: &format!("{}VertexBuffer{i}", desc.name),
                        size: data_size,
                        kind: BufferKind::Vertex,
                        usage: buffer.usage,
                    },
                )?;

                if let Some(data) = buffer.data.bytes {
                    native_buffer.write_data(data)?;
                }

                let target = native_buffer.kind.into_gl();
                server.gl.bind_buffer(target, Some(native_buffer.id));

                let mut offset = 0usize;
                for definition in buffer.attributes {
                    server.gl.vertex_attrib_pointer_f32(
                        definition.location,
                        definition.component_count as i32,
                        definition.kind.into_gl(),
                        definition.normalized,
                        buffer.data.element_size as i32,
                        offset as i32,
                    );
                    server
                        .gl
                        .vertex_attrib_divisor(definition.location, definition.divisor);
                    server.gl.enable_vertex_attrib_array(definition.location);

                    offset += definition.kind.size() * definition.component_count;

                    if offset > buffer.data.element_size {
                        return Err(FrameworkError::InvalidAttributeDescriptor);
                    }
                }

                buffers.push(native_buffer);
            }
        }

        server.set_vertex_array_object(None);

        Ok(GlGeometryBuffer {
            state: server.weak(),
            vertex_array_object: vao,
            buffers,
            element_buffer,
            element_count: Cell::new(element_count),
            element_kind: desc.elements.element_kind(),
            thread_mark: PhantomData,
        })
    }

    fn set_elements(&self, data: &[u8]) {
        self.state
            .upgrade()
            .unwrap()
            .set_vertex_array_object(Some(self.vertex_array_object));
        self.element_buffer.write_data(data).unwrap()
    }

    pub fn mode(&self) -> u32 {
        match self.element_kind {
            ElementKind::Triangle => glow::TRIANGLES,
            ElementKind::Line => glow::LINES,
            ElementKind::Point => glow::POINTS,
        }
    }
}

impl GpuGeometryBufferTrait for GlGeometryBuffer {
    fn set_buffer_data(&self, buffer: usize, data: &[u8]) {
        self.state
            .upgrade()
            .unwrap()
            .set_vertex_array_object(Some(self.vertex_array_object));
        self.buffers[buffer]
            .write_data(array_as_u8_slice(data))
            .unwrap();
    }

    fn element_count(&self) -> usize {
        self.element_count.get()
    }

    fn set_triangles(&self, triangles: &[TriangleDefinition]) {
        assert_eq!(self.element_kind, ElementKind::Triangle);
        self.element_count.set(triangles.len());
        self.set_elements(array_as_u8_slice(triangles));
    }

    fn set_lines(&self, lines: &[[u32; 2]]) {
        assert_eq!(self.element_kind, ElementKind::Line);
        self.element_count.set(lines.len());
        self.set_elements(array_as_u8_slice(lines));
    }

    fn set_points(&self, points: &[u32]) {
        assert_eq!(self.element_kind, ElementKind::Point);
        self.element_count.set(points.len());
        self.set_elements(array_as_u8_slice(points));
    }
}

impl Drop for GlGeometryBuffer {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            self.buffers.clear();
            state.delete_vertex_array_object(self.vertex_array_object);
        }
    }
}
