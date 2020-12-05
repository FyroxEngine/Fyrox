use crate::utils::log::MessageKind;
use crate::{
    core::scope_profile,
    renderer::{
        error::RendererError,
        framework::{
            gl::{
                self,
                types::{GLint, GLuint},
            },
            state::PipelineState,
        },
        TriangleDefinition,
    },
    utils::log::Log,
};
use std::{cell::Cell, ffi::c_void, marker::PhantomData, mem::size_of};

struct NativeBuffer {
    id: GLuint,
    kind: GeometryBufferKind,
    element_size: usize,
    size_bytes: usize,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

impl Drop for NativeBuffer {
    fn drop(&mut self) {
        unsafe {
            if self.id != 0 {
                gl::DeleteBuffers(1, &self.id);
            }
        }
    }
}

pub struct GeometryBuffer {
    vertex_array_object: GLuint,
    buffers: Vec<NativeBuffer>,
    element_buffer_object: GLuint,
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

    fn get_type(self) -> GLuint {
        match self {
            AttributeKind::Float
            | AttributeKind::Float2
            | AttributeKind::Float3
            | AttributeKind::Float4 => gl::FLOAT,

            AttributeKind::UnsignedByte
            | AttributeKind::UnsignedByte2
            | AttributeKind::UnsignedByte3
            | AttributeKind::UnsignedByte4 => gl::UNSIGNED_BYTE,

            AttributeKind::UnsignedShort
            | AttributeKind::UnsignedShort2
            | AttributeKind::UnsignedShort3
            | AttributeKind::UnsignedShort4 => gl::UNSIGNED_SHORT,

            AttributeKind::UnsignedInt
            | AttributeKind::UnsignedInt2
            | AttributeKind::UnsignedInt3
            | AttributeKind::UnsignedInt4 => gl::UNSIGNED_INT,
        }
    }

    fn length(self) -> GLint {
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

#[derive(Copy, Clone)]
#[repr(u32)]
pub enum GeometryBufferKind {
    StaticDraw = gl::STATIC_DRAW,
    DynamicDraw = gl::DYNAMIC_DRAW,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ElementKind {
    Triangle,
    Line,
}

impl ElementKind {
    fn index_per_element(self) -> usize {
        match self {
            ElementKind::Triangle => 3,
            ElementKind::Line => 2,
        }
    }
}

pub struct GeometryBufferBinding<'a> {
    buffer: &'a GeometryBuffer,
}

#[derive(Copy, Clone)]
pub struct DrawCallStatistics {
    pub triangles: usize,
}

impl<'a> GeometryBufferBinding<'a> {
    pub fn set_triangles(self, triangles: &[TriangleDefinition]) -> Self {
        scope_profile!();

        assert_eq!(self.buffer.element_kind, ElementKind::Triangle);
        self.buffer.element_count.set(triangles.len());

        let index_count = triangles.len() * 3;
        let size = (index_count * size_of::<u32>()) as isize;
        let data = triangles.as_ptr() as *const c_void;

        unsafe { self.set_elements(data, size) }

        self
    }

    pub fn set_lines(self, lines: &[[u32; 2]]) -> Self {
        scope_profile!();

        assert_eq!(self.buffer.element_kind, ElementKind::Line);
        self.buffer.element_count.set(lines.len());

        let index_count = lines.len() * 2;
        let size = (index_count * size_of::<u32>()) as isize;
        let data = lines.as_ptr() as *const c_void;

        unsafe { self.set_elements(data, size) }

        self
    }

    unsafe fn set_elements(&self, elements: *const c_void, size: isize) {
        scope_profile!();

        gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, size, elements, gl::DYNAMIC_DRAW);
    }

    pub fn draw_part(
        &self,
        offset: usize,
        count: usize,
    ) -> Result<DrawCallStatistics, RendererError> {
        scope_profile!();

        let last_triangle_index = offset + count;

        if last_triangle_index > self.buffer.element_count.get() {
            Err(RendererError::InvalidElementRange {
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

    fn mode(&self) -> GLuint {
        match self.buffer.element_kind {
            ElementKind::Triangle => gl::TRIANGLES,
            ElementKind::Line => gl::LINES,
        }
    }

    pub fn draw(&self) -> DrawCallStatistics {
        scope_profile!();

        let start_index = 0;
        let index_per_element = self.buffer.element_kind.index_per_element();
        let index_count = self.buffer.element_count.get() * index_per_element;

        unsafe { self.draw_internal(start_index, index_count) }

        DrawCallStatistics {
            triangles: self.buffer.element_count.get(),
        }
    }

    unsafe fn draw_internal(&self, start_index: usize, index_count: usize) {
        scope_profile!();

        if index_count > 0 {
            let indices = (start_index * size_of::<u32>()) as *const c_void;
            gl::DrawElements(self.mode(), index_count as i32, gl::UNSIGNED_INT, indices);
        }
    }

    pub fn draw_instances(&self, count: usize) -> DrawCallStatistics {
        let index_per_element = self.buffer.element_kind.index_per_element();
        let index_count = self.buffer.element_count.get() * index_per_element;
        if index_count > 0 {
            unsafe {
                gl::DrawElementsInstanced(
                    self.mode(),
                    index_count as i32,
                    gl::UNSIGNED_INT,
                    std::ptr::null(),
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
    pub fn set_buffer_data<T>(&mut self, state: &mut PipelineState, buffer: usize, data: &[T]) {
        scope_profile!();

        let buffer = &mut self.buffers[buffer];

        assert_eq!(buffer.element_size, size_of::<T>());

        state.set_vertex_buffer_object(buffer.id);

        let size = data.len() * size_of::<T>();
        let ptr = data.as_ptr() as *const c_void;
        let usage = buffer.kind as u32;

        unsafe {
            if buffer.size_bytes < size {
                gl::BufferData(gl::ARRAY_BUFFER, size as isize, ptr, usage);
            } else {
                gl::BufferSubData(gl::ARRAY_BUFFER, 0, size as isize, ptr);
            }
        }

        buffer.size_bytes = size;
    }

    pub fn bind(&self, state: &mut PipelineState) -> GeometryBufferBinding<'_> {
        scope_profile!();

        state.set_vertex_array_object(self.vertex_array_object);

        // Element buffer object binding is stored inside vertex array object, so
        // it does not modifies state.
        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.element_buffer_object);
        }

        GeometryBufferBinding { buffer: self }
    }
}

impl Drop for GeometryBuffer {
    fn drop(&mut self) {
        unsafe {
            Log::writeln(
                MessageKind::Information,
                format!(
                    "GL geometry buffer was destroyed - VAO: {}!",
                    self.vertex_array_object
                ),
            );

            self.buffers.clear();
            gl::DeleteBuffers(1, &self.element_buffer_object);
            gl::DeleteVertexArrays(1, &self.vertex_array_object);
        }
    }
}

pub struct BufferBuilder {
    element_size: usize,
    kind: GeometryBufferKind,
    attributes: Vec<AttributeDefinition>,
    data: *const c_void,
    data_size: usize,
}

impl BufferBuilder {
    pub fn new<T: Sized>(kind: GeometryBufferKind, data: Option<&[T]>) -> Self {
        let (data, data_size) = if let Some(data) = data {
            (
                data as *const _ as *const c_void,
                data.len() * size_of::<T>(),
            )
        } else {
            (std::ptr::null(), 0)
        };

        Self {
            kind,
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

    fn build(self, state: &mut PipelineState) -> Result<NativeBuffer, RendererError> {
        let mut vbo = 0;
        unsafe {
            gl::GenBuffers(1, &mut vbo);
        }

        state.set_vertex_buffer_object(vbo);

        if self.data_size > 0 {
            unsafe {
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    self.data_size as isize,
                    self.data,
                    self.kind as u32,
                );
            }
        }

        let native_buffer = NativeBuffer {
            id: vbo,
            kind: self.kind,
            element_size: self.element_size,
            size_bytes: self.data_size,
            thread_mark: Default::default(),
        };

        let mut offset = 0;
        for definition in self.attributes {
            let size = definition.kind.length();
            let type_ = definition.kind.get_type();
            let normalized = if definition.normalized {
                gl::TRUE
            } else {
                gl::FALSE
            };
            let stride = self.element_size as i32;
            let pointer = offset as *const c_void;

            unsafe {
                gl::VertexAttribPointer(
                    definition.location,
                    size,
                    type_,
                    normalized,
                    stride,
                    pointer,
                );
                gl::VertexAttribDivisor(definition.location, definition.divisor);
                gl::EnableVertexAttribArray(definition.location);

                offset += definition.kind.size_bytes();

                if offset > self.element_size {
                    state.set_vertex_buffer_object(0);
                    return Err(RendererError::InvalidAttributeDescriptor);
                }
            }
        }

        Ok(native_buffer)
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

    pub fn build(self, state: &mut PipelineState) -> Result<GeometryBuffer, RendererError> {
        scope_profile!();

        let mut vao = 0;
        let mut ebo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut ebo);
        }

        state.set_vertex_array_object(vao);

        let mut buffers = Vec::new();
        for builder in self.buffers {
            buffers.push(builder.build(state)?);
        }

        Ok(GeometryBuffer {
            vertex_array_object: vao,
            buffers,
            element_buffer_object: ebo,
            element_count: Cell::new(0),
            element_kind: self.element_kind,
            thread_mark: PhantomData,
        })
    }
}
