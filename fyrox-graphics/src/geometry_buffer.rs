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

use crate::{
    buffer::{Buffer, BufferKind, BufferUsage},
    core::{array_as_u8_slice, math::TriangleDefinition},
    error::FrameworkError,
    gl::{buffer::GlBuffer, server::GlGraphicsServer, ToGlConstant},
    ElementKind, ElementRange,
};
use bytemuck::Pod;
use glow::HasContext;
use std::{cell::Cell, marker::PhantomData, mem::size_of, rc::Weak};

pub struct GeometryBuffer {
    state: Weak<GlGraphicsServer>,
    vertex_array_object: glow::VertexArray,
    buffers: Vec<GlBuffer>,
    element_buffer: GlBuffer,
    element_count: Cell<usize>,
    element_kind: ElementKind,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub enum AttributeKind {
    Float,
    UnsignedByte,
    UnsignedShort,
    UnsignedInt,
}

pub struct AttributeDefinition {
    pub location: u32,
    pub kind: AttributeKind,
    pub component_count: usize,
    pub normalized: bool,
    pub divisor: u32,
}

impl AttributeKind {
    pub fn size(self) -> usize {
        match self {
            AttributeKind::Float => size_of::<f32>(),
            AttributeKind::UnsignedByte => size_of::<u8>(),
            AttributeKind::UnsignedShort => size_of::<u16>(),
            AttributeKind::UnsignedInt => size_of::<u32>(),
        }
    }

    fn gl_type(self) -> u32 {
        match self {
            AttributeKind::Float => glow::FLOAT,
            AttributeKind::UnsignedByte => glow::UNSIGNED_BYTE,
            AttributeKind::UnsignedShort => glow::UNSIGNED_SHORT,
            AttributeKind::UnsignedInt => glow::UNSIGNED_INT,
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct DrawCallStatistics {
    pub triangles: usize,
}

pub struct VertexBufferData<'a> {
    pub element_size: usize,
    pub bytes: Option<&'a [u8]>,
}

impl<'a> VertexBufferData<'a> {
    pub fn new<T: Pod>(vertices: Option<&'a [T]>) -> Self {
        Self {
            element_size: size_of::<T>(),
            bytes: vertices.map(|v| array_as_u8_slice(v)),
        }
    }
}

pub struct VertexBufferDescriptor<'a> {
    pub usage: BufferUsage,
    pub attributes: &'a [AttributeDefinition],
    pub data: VertexBufferData<'a>,
}

pub struct GeometryBufferDescriptor<'a> {
    pub element_kind: ElementKind,
    pub buffers: &'a [VertexBufferDescriptor<'a>],
}

impl GeometryBuffer {
    pub fn new(
        server: &GlGraphicsServer,
        desc: GeometryBufferDescriptor,
    ) -> Result<Self, FrameworkError> {
        let vao = unsafe { server.gl.create_vertex_array()? };

        server.set_vertex_array_object(Some(vao));

        let element_buffer = GlBuffer::new(server, 0, BufferKind::Index, BufferUsage::StaticDraw)?;

        let mut buffers = Vec::new();
        for buffer in desc.buffers {
            unsafe {
                let data_size = buffer.data.bytes.map(|bytes| bytes.len()).unwrap_or(0);

                let native_buffer =
                    GlBuffer::new(server, data_size, BufferKind::Vertex, buffer.usage)?;

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
                        definition.kind.gl_type(),
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

        Ok(GeometryBuffer {
            state: server.weak(),
            vertex_array_object: vao,
            buffers,
            element_buffer,
            element_count: Cell::new(0),
            element_kind: desc.element_kind,
            thread_mark: PhantomData,
        })
    }

    pub fn set_buffer_data<T: bytemuck::Pod>(&mut self, buffer: usize, data: &[T]) {
        self.state
            .upgrade()
            .unwrap()
            .set_vertex_array_object(Some(self.vertex_array_object));
        self.buffers[buffer]
            .write_data(array_as_u8_slice(data))
            .unwrap();
    }

    pub fn element_count(&self) -> usize {
        self.element_count.get()
    }

    pub fn set_triangles(&self, triangles: &[TriangleDefinition]) {
        assert_eq!(self.element_kind, ElementKind::Triangle);
        self.element_count.set(triangles.len());
        self.set_elements(array_as_u8_slice(triangles));
    }

    pub fn set_lines(&self, lines: &[[u32; 2]]) {
        assert_eq!(self.element_kind, ElementKind::Line);
        self.element_count.set(lines.len());
        self.set_elements(array_as_u8_slice(lines));
    }

    fn set_elements(&self, data: &[u8]) {
        self.state
            .upgrade()
            .unwrap()
            .set_vertex_array_object(Some(self.vertex_array_object));
        self.element_buffer.write_data(data).unwrap()
    }

    pub fn draw(&self, element_range: ElementRange) -> Result<DrawCallStatistics, FrameworkError> {
        let server = self.state.upgrade().unwrap();

        let (offset, count) = match element_range {
            ElementRange::Full => (0, self.element_count.get()),
            ElementRange::Specific { offset, count } => (offset, count),
        };

        let last_triangle_index = offset + count;

        if last_triangle_index > self.element_count.get() {
            Err(FrameworkError::InvalidElementRange {
                start: offset,
                end: last_triangle_index,
                total: self.element_count.get(),
            })
        } else {
            let index_per_element = self.element_kind.index_per_element();
            let start_index = offset * index_per_element;
            let index_count = count * index_per_element;

            unsafe {
                if index_count > 0 {
                    server.set_vertex_array_object(Some(self.vertex_array_object));

                    let indices = (start_index * size_of::<u32>()) as i32;
                    server.gl.draw_elements(
                        self.mode(),
                        index_count as i32,
                        glow::UNSIGNED_INT,
                        indices,
                    );
                }
            }

            Ok(DrawCallStatistics { triangles: count })
        }
    }

    fn mode(&self) -> u32 {
        match self.element_kind {
            ElementKind::Triangle => glow::TRIANGLES,
            ElementKind::Line => glow::LINES,
            ElementKind::Point => glow::POINTS,
        }
    }

    pub fn draw_instances(&self, count: usize) -> DrawCallStatistics {
        let server = self.state.upgrade().unwrap();

        let index_per_element = self.element_kind.index_per_element();
        let index_count = self.element_count.get() * index_per_element;
        if index_count > 0 {
            unsafe {
                server.set_vertex_array_object(Some(self.vertex_array_object));

                server.gl.draw_elements_instanced(
                    self.mode(),
                    index_count as i32,
                    glow::UNSIGNED_INT,
                    0,
                    count as i32,
                )
            }
        }
        DrawCallStatistics {
            triangles: self.element_count.get() * count,
        }
    }
}

impl Drop for GeometryBuffer {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            unsafe {
                self.buffers.clear();
                state.gl.delete_vertex_array(self.vertex_array_object);
            }
        }
    }
}
