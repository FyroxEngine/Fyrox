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
    buffer::{Buffer, BufferKind, BufferUsage},
    core::{array_as_u8_slice, math::TriangleDefinition},
    error::FrameworkError,
    gl::buffer::GlBuffer,
    ElementKind, ElementRange,
};
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
    Float2,
    Float3,
    Float4,

    UnsignedByte,
    UnsignedByte2,
    UnsignedByte3,
    UnsignedByte4,

    UnsignedShort,
    UnsignedShort2,
    UnsignedShort3,
    UnsignedShort4,

    UnsignedInt,
    UnsignedInt2,
    UnsignedInt3,
    UnsignedInt4,
}

pub struct AttributeDefinition {
    pub location: u32,
    pub kind: AttributeKind,
    pub normalized: bool,
    pub divisor: u32,
}

impl AttributeKind {
    pub fn size_bytes(self) -> usize {
        match self {
            AttributeKind::Float => size_of::<f32>(),
            AttributeKind::Float2 => size_of::<f32>() * 2,
            AttributeKind::Float3 => size_of::<f32>() * 3,
            AttributeKind::Float4 => size_of::<f32>() * 4,

            AttributeKind::UnsignedByte => size_of::<u8>(),
            AttributeKind::UnsignedByte2 => size_of::<u8>() * 2,
            AttributeKind::UnsignedByte3 => size_of::<u8>() * 3,
            AttributeKind::UnsignedByte4 => size_of::<u8>() * 4,

            AttributeKind::UnsignedShort => size_of::<u16>(),
            AttributeKind::UnsignedShort2 => size_of::<u16>() * 2,
            AttributeKind::UnsignedShort3 => size_of::<u16>() * 3,
            AttributeKind::UnsignedShort4 => size_of::<u16>() * 4,

            AttributeKind::UnsignedInt => size_of::<u32>(),
            AttributeKind::UnsignedInt2 => size_of::<u32>() * 2,
            AttributeKind::UnsignedInt3 => size_of::<u32>() * 3,
            AttributeKind::UnsignedInt4 => size_of::<u32>() * 4,
        }
    }

    fn get_type(self) -> u32 {
        match self {
            AttributeKind::Float
            | AttributeKind::Float2
            | AttributeKind::Float3
            | AttributeKind::Float4 => glow::FLOAT,

            AttributeKind::UnsignedByte
            | AttributeKind::UnsignedByte2
            | AttributeKind::UnsignedByte3
            | AttributeKind::UnsignedByte4 => glow::UNSIGNED_BYTE,

            AttributeKind::UnsignedShort
            | AttributeKind::UnsignedShort2
            | AttributeKind::UnsignedShort3
            | AttributeKind::UnsignedShort4 => glow::UNSIGNED_SHORT,

            AttributeKind::UnsignedInt
            | AttributeKind::UnsignedInt2
            | AttributeKind::UnsignedInt3
            | AttributeKind::UnsignedInt4 => glow::UNSIGNED_INT,
        }
    }

    fn length(self) -> usize {
        match self {
            AttributeKind::Float
            | AttributeKind::UnsignedByte
            | AttributeKind::UnsignedShort
            | AttributeKind::UnsignedInt => 1,

            AttributeKind::Float2
            | AttributeKind::UnsignedByte2
            | AttributeKind::UnsignedShort2
            | AttributeKind::UnsignedInt2 => 2,

            AttributeKind::Float3
            | AttributeKind::UnsignedByte3
            | AttributeKind::UnsignedShort3
            | AttributeKind::UnsignedInt3 => 3,

            AttributeKind::Float4
            | AttributeKind::UnsignedByte4
            | AttributeKind::UnsignedShort4
            | AttributeKind::UnsignedInt4 => 4,
        }
    }
}

pub struct GeometryBufferBinding<'a> {
    state: &'a GlGraphicsServer,
    buffer: &'a GeometryBuffer,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct DrawCallStatistics {
    pub triangles: usize,
}

impl<'a> GeometryBufferBinding<'a> {
    pub fn set_triangles(self, triangles: &[TriangleDefinition]) -> Self {
        assert_eq!(self.buffer.element_kind, ElementKind::Triangle);
        self.buffer.element_count.set(triangles.len());
        self.set_elements(array_as_u8_slice(triangles));
        self
    }

    pub fn set_lines(self, lines: &[[u32; 2]]) -> Self {
        assert_eq!(self.buffer.element_kind, ElementKind::Line);
        self.buffer.element_count.set(lines.len());
        self.set_elements(array_as_u8_slice(lines));
        self
    }

    fn set_elements(&self, data: &[u8]) {
        self.buffer.element_buffer.write_data(data).unwrap()
    }

    pub fn draw(&self, element_range: ElementRange) -> Result<DrawCallStatistics, FrameworkError> {
        let (offset, count) = match element_range {
            ElementRange::Full => (0, self.buffer.element_count.get()),
            ElementRange::Specific { offset, count } => (offset, count),
        };

        let last_triangle_index = offset + count;

        if last_triangle_index > self.buffer.element_count.get() {
            Err(FrameworkError::InvalidElementRange {
                start: offset,
                end: last_triangle_index,
                total: self.buffer.element_count.get(),
            })
        } else {
            let index_per_element = self.buffer.element_kind.index_per_element();
            let start_index = offset * index_per_element;
            let index_count = count * index_per_element;

            unsafe {
                self.draw_internal(start_index, index_count);
            }

            Ok(DrawCallStatistics { triangles: count })
        }
    }

    fn mode(&self) -> u32 {
        match self.buffer.element_kind {
            ElementKind::Triangle => glow::TRIANGLES,
            ElementKind::Line => glow::LINES,
            ElementKind::Point => glow::POINTS,
        }
    }

    unsafe fn draw_internal(&self, start_index: usize, index_count: usize) {
        if index_count > 0 {
            let indices = (start_index * size_of::<u32>()) as i32;
            self.state.gl.draw_elements(
                self.mode(),
                index_count as i32,
                glow::UNSIGNED_INT,
                indices,
            );
        }
    }

    pub fn draw_instances(&self, count: usize) -> DrawCallStatistics {
        let index_per_element = self.buffer.element_kind.index_per_element();
        let index_count = self.buffer.element_count.get() * index_per_element;
        if index_count > 0 {
            unsafe {
                self.state.gl.draw_elements_instanced(
                    self.mode(),
                    index_count as i32,
                    glow::UNSIGNED_INT,
                    0,
                    count as i32,
                )
            }
        }
        DrawCallStatistics {
            triangles: self.buffer.element_count.get() * count,
        }
    }
}

impl GeometryBuffer {
    pub fn set_buffer_data<T: bytemuck::Pod>(&mut self, buffer: usize, data: &[T]) {
        self.buffers[buffer]
            .write_data(array_as_u8_slice(data))
            .unwrap();
    }

    pub fn bind<'a>(&'a self, state: &'a GlGraphicsServer) -> GeometryBufferBinding<'a> {
        state.set_vertex_array_object(Some(self.vertex_array_object));

        GeometryBufferBinding {
            state,
            buffer: self,
        }
    }

    pub fn element_count(&self) -> usize {
        self.element_count.get()
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

pub struct BufferBuilder {
    pub element_size: usize,
    pub usage: BufferUsage,
    pub attributes: Vec<AttributeDefinition>,
    pub data: *const u8,
    pub data_size: usize,
}

impl BufferBuilder {
    pub fn new<T: Sized>(usage: BufferUsage, data: Option<&[T]>) -> Self {
        let (data, data_size) = if let Some(data) = data {
            (data as *const _ as *const u8, std::mem::size_of_val(data))
        } else {
            (std::ptr::null(), 0)
        };

        Self {
            usage,
            attributes: Default::default(),
            element_size: size_of::<T>(),
            data,
            data_size,
        }
    }

    pub fn with_attribute(mut self, attribute: AttributeDefinition) -> Self {
        self.attributes.push(attribute);
        self
    }

    fn build(self, server: &GlGraphicsServer) -> Result<GlBuffer, FrameworkError> {
        unsafe {
            let native_buffer =
                GlBuffer::new(server, self.data_size, BufferKind::Vertex, self.usage)?;
            if self.data_size > 0 {
                let data = std::slice::from_raw_parts(self.data, self.data_size);
                native_buffer.write_data(data)?;
            }

            let target = native_buffer.kind.into_gl();
            server.gl.bind_buffer(target, Some(native_buffer.id));

            let mut offset = 0usize;
            for definition in self.attributes {
                server.gl.vertex_attrib_pointer_f32(
                    definition.location,
                    definition.kind.length() as i32,
                    definition.kind.get_type(),
                    definition.normalized,
                    self.element_size as i32,
                    offset as i32,
                );
                server
                    .gl
                    .vertex_attrib_divisor(definition.location, definition.divisor);
                server.gl.enable_vertex_attrib_array(definition.location);

                offset += definition.kind.size_bytes();

                if offset > self.element_size {
                    return Err(FrameworkError::InvalidAttributeDescriptor);
                }
            }

            Ok(native_buffer)
        }
    }
}

pub struct GeometryBufferBuilder {
    element_kind: ElementKind,
    buffers: Vec<BufferBuilder>,
}

impl GeometryBufferBuilder {
    pub fn new(element_kind: ElementKind) -> Self {
        Self {
            element_kind,
            buffers: Default::default(),
        }
    }

    pub fn with_buffer_builder(mut self, builder: BufferBuilder) -> Self {
        self.buffers.push(builder);
        self
    }

    pub fn build(self, server: &GlGraphicsServer) -> Result<GeometryBuffer, FrameworkError> {
        let vao = unsafe { server.gl.create_vertex_array()? };

        server.set_vertex_array_object(Some(vao));

        let element_buffer = GlBuffer::new(server, 0, BufferKind::Index, BufferUsage::StaticDraw)?;

        let mut buffers = Vec::new();
        for builder in self.buffers {
            buffers.push(builder.build(server)?);
        }

        server.set_vertex_array_object(None);

        Ok(GeometryBuffer {
            state: server.weak(),
            vertex_array_object: vao,
            buffers,
            element_buffer,
            element_count: Cell::new(0),
            element_kind: self.element_kind,
            thread_mark: PhantomData,
        })
    }
}
