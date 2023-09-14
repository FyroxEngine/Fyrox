use crate::scene::mesh::surface::SurfaceData;
use crate::{
    core::{math::TriangleDefinition, scope_profile},
    renderer::framework::{error::FrameworkError, state::PipelineState},
    scene::mesh::buffer::{VertexAttributeDataType, VertexBuffer},
    utils::array_as_u8_slice,
};
use glow::HasContext;
use std::{cell::Cell, marker::PhantomData, mem::size_of};

struct NativeBuffer {
    state: *mut PipelineState,
    id: glow::Buffer,
    kind: GeometryBufferKind,
    element_size: usize,
    size_bytes: usize,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

impl Drop for NativeBuffer {
    fn drop(&mut self) {
        unsafe {
            (*self.state).gl.delete_buffer(self.id);
        }
    }
}

pub struct GeometryBuffer {
    state: *mut PipelineState,
    vertex_array_object: glow::VertexArray,
    buffers: Vec<NativeBuffer>,
    element_buffer_object: glow::Buffer,
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

impl From<(VertexAttributeDataType, u8)> for AttributeKind {
    fn from(t: (VertexAttributeDataType, u8)) -> Self {
        match t {
            (VertexAttributeDataType::F32, 1) => Self::Float,
            (VertexAttributeDataType::F32, 2) => Self::Float2,
            (VertexAttributeDataType::F32, 3) => Self::Float3,
            (VertexAttributeDataType::F32, 4) => Self::Float4,
            (VertexAttributeDataType::U32, 1) => Self::UnsignedInt,
            (VertexAttributeDataType::U32, 2) => Self::UnsignedInt2,
            (VertexAttributeDataType::U32, 3) => Self::UnsignedInt3,
            (VertexAttributeDataType::U32, 4) => Self::UnsignedInt4,
            (VertexAttributeDataType::U16, 1) => Self::UnsignedShort,
            (VertexAttributeDataType::U16, 2) => Self::UnsignedShort2,
            (VertexAttributeDataType::U16, 3) => Self::UnsignedShort3,
            (VertexAttributeDataType::U16, 4) => Self::UnsignedShort4,
            (VertexAttributeDataType::U8, 1) => Self::UnsignedByte,
            (VertexAttributeDataType::U8, 2) => Self::UnsignedByte2,
            (VertexAttributeDataType::U8, 3) => Self::UnsignedByte3,
            (VertexAttributeDataType::U8, 4) => Self::UnsignedByte4,
            _ => unreachable!(),
        }
    }
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
            Self::Float => size_of::<f32>(),
            Self::Float2 => size_of::<f32>() * 2,
            Self::Float3 => size_of::<f32>() * 3,
            Self::Float4 => size_of::<f32>() * 4,

            Self::UnsignedByte => size_of::<u8>(),
            Self::UnsignedByte2 => size_of::<u8>() * 2,
            Self::UnsignedByte3 => size_of::<u8>() * 3,
            Self::UnsignedByte4 => size_of::<u8>() * 4,

            Self::UnsignedShort => size_of::<u16>(),
            Self::UnsignedShort2 => size_of::<u16>() * 2,
            Self::UnsignedShort3 => size_of::<u16>() * 3,
            Self::UnsignedShort4 => size_of::<u16>() * 4,

            Self::UnsignedInt => size_of::<u32>(),
            Self::UnsignedInt2 => size_of::<u32>() * 2,
            Self::UnsignedInt3 => size_of::<u32>() * 3,
            Self::UnsignedInt4 => size_of::<u32>() * 4,
        }
    }

    fn get_type(self) -> u32 {
        match self {
            Self::Float | Self::Float2 | Self::Float3 | Self::Float4 => glow::FLOAT,

            Self::UnsignedByte
            | Self::UnsignedByte2
            | Self::UnsignedByte3
            | Self::UnsignedByte4 => glow::UNSIGNED_BYTE,

            Self::UnsignedShort
            | Self::UnsignedShort2
            | Self::UnsignedShort3
            | Self::UnsignedShort4 => glow::UNSIGNED_SHORT,

            Self::UnsignedInt | Self::UnsignedInt2 | Self::UnsignedInt3 | Self::UnsignedInt4 => {
                glow::UNSIGNED_INT
            }
        }
    }

    fn length(self) -> usize {
        match self {
            Self::Float | Self::UnsignedByte | Self::UnsignedShort | Self::UnsignedInt => 1,

            Self::Float2 | Self::UnsignedByte2 | Self::UnsignedShort2 | Self::UnsignedInt2 => 2,

            Self::Float3 | Self::UnsignedByte3 | Self::UnsignedShort3 | Self::UnsignedInt3 => 3,

            Self::Float4 | Self::UnsignedByte4 | Self::UnsignedShort4 | Self::UnsignedInt4 => 4,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(u32)]
pub enum GeometryBufferKind {
    StaticDraw = glow::STATIC_DRAW,
    DynamicDraw = glow::DYNAMIC_DRAW,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ElementKind {
    Triangle,
    Line,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ElementRange {
    Full,
    Specific { offset: usize, count: usize },
}

impl Default for ElementRange {
    fn default() -> Self {
        Self::Full
    }
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
    state: &'a mut PipelineState,
    buffer: &'a GeometryBuffer,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct DrawCallStatistics {
    pub triangles: usize,
}

impl<'a> GeometryBufferBinding<'a> {
    pub fn set_triangles(self, triangles: &[TriangleDefinition]) -> Self {
        scope_profile!();

        assert_eq!(self.buffer.element_kind, ElementKind::Triangle);
        self.buffer.element_count.set(triangles.len());

        unsafe { self.set_elements(array_as_u8_slice(triangles)) }

        self
    }

    pub fn set_lines(self, lines: &[[u32; 2]]) -> Self {
        scope_profile!();

        assert_eq!(self.buffer.element_kind, ElementKind::Line);
        self.buffer.element_count.set(lines.len());

        unsafe {
            self.set_elements(array_as_u8_slice(lines));
        }

        self
    }

    unsafe fn set_elements(&self, data: &[u8]) {
        scope_profile!();

        self.state
            .gl
            .buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, data, glow::DYNAMIC_DRAW);
    }

    pub fn draw(&self, element_range: ElementRange) -> Result<DrawCallStatistics, FrameworkError> {
        scope_profile!();

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
        }
    }

    unsafe fn draw_internal(&self, start_index: usize, index_count: usize) {
        scope_profile!();

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
    pub fn from_surface_data(
        data: &SurfaceData,
        kind: GeometryBufferKind,
        state: &mut PipelineState,
    ) -> Self {
        let geometry_buffer = GeometryBufferBuilder::new(ElementKind::Triangle)
            .with_buffer_builder(BufferBuilder::from_vertex_buffer(&data.vertex_buffer, kind))
            .build(state)
            .unwrap();

        geometry_buffer
            .bind(state)
            .set_triangles(data.geometry_buffer.triangles_ref());

        geometry_buffer
    }

    pub fn set_buffer_data<T>(&mut self, state: &mut PipelineState, buffer: usize, data: &[T]) {
        scope_profile!();

        let buffer = &mut self.buffers[buffer];

        assert_eq!(buffer.element_size % size_of::<T>(), 0);

        state.set_vertex_buffer_object(Some(buffer.id));

        let size = std::mem::size_of_val(data);
        let usage = buffer.kind as u32;

        unsafe {
            if buffer.size_bytes < size || size == 0 {
                state
                    .gl
                    .buffer_data_u8_slice(glow::ARRAY_BUFFER, array_as_u8_slice(data), usage);
            } else {
                state
                    .gl
                    .buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, array_as_u8_slice(data));
            }
        }

        buffer.size_bytes = size;
    }

    pub fn bind<'a>(&'a self, state: &'a mut PipelineState) -> GeometryBufferBinding<'a> {
        scope_profile!();

        state.set_vertex_array_object(Some(self.vertex_array_object));

        // Element buffer object binding is stored inside vertex array object, so
        // it does not modifies state.
        unsafe {
            state
                .gl
                .bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.element_buffer_object));
        }

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
        unsafe {
            self.buffers.clear();

            (*self.state).gl.delete_buffer(self.element_buffer_object);
            (*self.state)
                .gl
                .delete_vertex_array(self.vertex_array_object);
        }
    }
}

pub struct BufferBuilder {
    element_size: usize,
    kind: GeometryBufferKind,
    attributes: Vec<AttributeDefinition>,
    data: *const u8,
    data_size: usize,
}

impl BufferBuilder {
    pub fn new<T: Sized>(kind: GeometryBufferKind, data: Option<&[T]>) -> Self {
        let (data, data_size) = if let Some(data) = data {
            (data as *const _ as *const u8, std::mem::size_of_val(data))
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

    pub fn from_vertex_buffer(buffer: &VertexBuffer, kind: GeometryBufferKind) -> Self {
        Self {
            element_size: buffer.vertex_size() as usize,
            kind,
            attributes: buffer
                .layout()
                .iter()
                .map(|a| AttributeDefinition {
                    location: a.shader_location as u32,
                    kind: (a.data_type, a.size).into(),
                    normalized: false,
                    divisor: 0,
                })
                .collect(),
            data: buffer.raw_data().as_ptr(),
            data_size: buffer.raw_data().len(),
        }
    }

    pub fn with_attribute(mut self, attribute: AttributeDefinition) -> Self {
        self.attributes.push(attribute);
        self
    }

    fn build(self, state: &mut PipelineState) -> Result<NativeBuffer, FrameworkError> {
        let vbo = unsafe { state.gl.create_buffer()? };

        state.set_vertex_buffer_object(Some(vbo));

        if self.data_size > 0 {
            unsafe {
                state.gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    std::slice::from_raw_parts(self.data, self.data_size),
                    self.kind as u32,
                );
            }
        }

        let native_buffer = NativeBuffer {
            state,
            id: vbo,
            kind: self.kind,
            element_size: self.element_size,
            size_bytes: self.data_size,
            thread_mark: Default::default(),
        };

        let mut offset = 0usize;
        for definition in self.attributes {
            unsafe {
                state.gl.vertex_attrib_pointer_f32(
                    definition.location,
                    definition.kind.length() as i32,
                    definition.kind.get_type(),
                    definition.normalized,
                    self.element_size as i32,
                    offset as i32,
                );
                state
                    .gl
                    .vertex_attrib_divisor(definition.location, definition.divisor);
                state.gl.enable_vertex_attrib_array(definition.location);

                offset += definition.kind.size_bytes();

                if offset > self.element_size {
                    state.set_vertex_buffer_object(Default::default());
                    return Err(FrameworkError::InvalidAttributeDescriptor);
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

    pub fn build(self, state: &mut PipelineState) -> Result<GeometryBuffer, FrameworkError> {
        scope_profile!();

        let vao = unsafe { state.gl.create_vertex_array()? };
        let ebo = unsafe { state.gl.create_buffer()? };

        state.set_vertex_array_object(Some(vao));

        let mut buffers = Vec::new();
        for builder in self.buffers {
            buffers.push(builder.build(state)?);
        }

        Ok(GeometryBuffer {
            state,
            vertex_array_object: vao,
            buffers,
            element_buffer_object: ebo,
            element_count: Cell::new(0),
            element_kind: self.element_kind,
            thread_mark: PhantomData,
        })
    }
}
