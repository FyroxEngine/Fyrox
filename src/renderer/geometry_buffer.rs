use std::{
    marker::PhantomData,
    mem::size_of,
    ffi::c_void,
    cell::Cell,
};
use crate::renderer::{
    error::RendererError,
    gl, gl::types::{GLuint, GLint},
    TriangleDefinition,
};

/// Safe wrapper over OpenGL's Vertex Array Objects for interleaved vertices (where
/// position, normal, etc. stored together, not in separate arrays)
/// WARNING: T must have #[repr(C)] attribute!
pub struct GeometryBuffer<T> {
    vertex_array_object: GLuint,
    vertex_buffer_object: GLuint,
    element_buffer_object: GLuint,
    meta: PhantomData<T>,
    kind: GeometryBufferKind,
    triangle_count: Cell<usize>,
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
    pub kind: AttributeKind,
    pub normalized: bool,
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

    fn get_type(self) -> GLuint {
        match self {
            AttributeKind::Float => gl::FLOAT,
            AttributeKind::Float2 => gl::FLOAT,
            AttributeKind::Float3 => gl::FLOAT,
            AttributeKind::Float4 => gl::FLOAT,

            AttributeKind::UnsignedByte => gl::UNSIGNED_BYTE,
            AttributeKind::UnsignedByte2 => gl::UNSIGNED_BYTE,
            AttributeKind::UnsignedByte3 => gl::UNSIGNED_BYTE,
            AttributeKind::UnsignedByte4 => gl::UNSIGNED_BYTE,

            AttributeKind::UnsignedShort => gl::UNSIGNED_SHORT,
            AttributeKind::UnsignedShort2 => gl::UNSIGNED_SHORT,
            AttributeKind::UnsignedShort3 => gl::UNSIGNED_SHORT,
            AttributeKind::UnsignedShort4 => gl::UNSIGNED_SHORT,

            AttributeKind::UnsignedInt => gl::UNSIGNED_INT,
            AttributeKind::UnsignedInt2 => gl::UNSIGNED_INT,
            AttributeKind::UnsignedInt3 => gl::UNSIGNED_INT,
            AttributeKind::UnsignedInt4 => gl::UNSIGNED_INT,
        }
    }

    fn length(self) -> GLint {
        match self {
            AttributeKind::Float => 1,
            AttributeKind::Float2 => 2,
            AttributeKind::Float3 => 3,
            AttributeKind::Float4 => 4,

            AttributeKind::UnsignedByte => 1,
            AttributeKind::UnsignedByte2 => 2,
            AttributeKind::UnsignedByte3 => 3,
            AttributeKind::UnsignedByte4 => 4,

            AttributeKind::UnsignedShort => 1,
            AttributeKind::UnsignedShort2 => 2,
            AttributeKind::UnsignedShort3 => 3,
            AttributeKind::UnsignedShort4 => 4,

            AttributeKind::UnsignedInt => 1,
            AttributeKind::UnsignedInt2 => 2,
            AttributeKind::UnsignedInt3 => 3,
            AttributeKind::UnsignedInt4 => 4,
        }
    }
}

pub enum GeometryBufferKind {
    StaticDraw,
    DynamicDraw,
}

impl<T> GeometryBuffer<T> where T: Sized {
    pub fn new(kind: GeometryBufferKind) -> Self {
        unsafe {
            let mut vao = 0;
            gl::GenVertexArrays(1, &mut vao);

            let mut vbo = 0;
            gl::GenBuffers(1, &mut vbo);

            let mut ebo = 0;
            gl::GenBuffers(1, &mut ebo);

            Self {
                vertex_array_object: vao,
                vertex_buffer_object: vbo,
                element_buffer_object: ebo,
                meta: PhantomData,
                kind,
                triangle_count: Cell::new(0),
            }
        }
    }

    fn get_usage(&self) -> GLuint {
        match self.kind {
            GeometryBufferKind::StaticDraw => gl::STATIC_DRAW,
            GeometryBufferKind::DynamicDraw => gl::DYNAMIC_DRAW,
        }
    }

    pub fn set_vertices(&self, vertices: &[T]) {
        let size = (vertices.len() * size_of::<T>()) as isize;
        let data = vertices.as_ptr() as *const c_void;
        let usage = self.get_usage();

        unsafe {
            gl::BindVertexArray(self.vertex_array_object);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer_object);

            gl::BufferData(gl::ARRAY_BUFFER, size, data, usage);

            gl::BindVertexArray(0);
        }
    }

    pub fn describe_attributes(&self, definitions: Vec<AttributeDefinition>) -> Result<(), RendererError> {
        unsafe {
            gl::BindVertexArray(self.vertex_array_object);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer_object);
        }

        let vertex_size = size_of::<T>();
        let mut offset = 0;
        for (index, definition) in definitions.iter().enumerate() {
            let index = index as u32;
            let size = definition.kind.length();
            let type_ = definition.kind.get_type();
            let normalized = if definition.normalized { gl::TRUE } else { gl::FALSE };
            let stride = vertex_size as i32;
            let pointer = offset as *const c_void;

            unsafe {
                gl::VertexAttribPointer(index, size, type_, normalized, stride, pointer);
                gl::EnableVertexAttribArray(index);
            }

            offset += definition.kind.size_bytes();

            if offset > vertex_size {
                return Err(RendererError::InvalidAttributeDescriptor);
            }
        }

        unsafe {
            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        Ok(())
    }

    pub fn set_triangles(&self, triangles: &[TriangleDefinition]) {
        self.triangle_count.set(triangles.len());

        let index_count = triangles.len() * 3;
        let size = (index_count * size_of::<u32>()) as isize;
        let data = triangles.as_ptr() as *const c_void;
        let usage = self.get_usage();

        unsafe {
            gl::BindVertexArray(self.vertex_array_object);

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.element_buffer_object);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, size, data, usage);

            gl::BindVertexArray(0);
        }
    }

    pub fn draw_part(&self, start_triangle: usize, triangle_count: usize) -> Result<usize, RendererError> {
        let last_triangle_index = start_triangle + triangle_count;

        if last_triangle_index > self.triangle_count.get() {
            Err(RendererError::InvalidTriangleRange {
                start: start_triangle,
                end: last_triangle_index,
                total: self.triangle_count.get()
            })
        } else {
            let start_index = start_triangle * 3;
            let index_count = triangle_count * 3;

            unsafe { self.draw_internal(start_index, index_count); }

            Ok(triangle_count)
        }
    }

    pub fn draw(&self) -> usize {
        let start_index = 0;
        let index_count = self.triangle_count.get() * 3;

        unsafe { self.draw_internal(start_index, index_count) }

        self.triangle_count.get()
    }

    unsafe fn draw_internal(&self, start_index: usize, index_count: usize) {
        if index_count > 0 {
            let indices = (start_index * size_of::<u32>()) as *const c_void;

            gl::BindVertexArray(self.vertex_array_object);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.element_buffer_object);
            gl::DrawElements(gl::TRIANGLES, index_count as i32, gl::UNSIGNED_INT, indices);
        }
    }
}

impl<T> Drop for GeometryBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vertex_buffer_object);
            gl::DeleteBuffers(1, &self.element_buffer_object);
            gl::DeleteVertexArrays(1, &self.vertex_array_object);
        }
    }
}