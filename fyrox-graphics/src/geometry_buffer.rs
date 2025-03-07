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

//! Geometry buffer is a mesh buffer, that could contain multiple vertex buffers and only one
//! element buffer.

#![warn(missing_docs)]

use crate::{
    buffer::BufferUsage,
    core::{array_as_u8_slice, math::TriangleDefinition},
    define_shared_wrapper, ElementKind,
};
use bytemuck::Pod;
use fyrox_core::define_as_any_trait;
use std::mem::size_of;

/// Attribute kind of a vertex.
#[derive(Copy, Clone)]
#[allow(dead_code)]
pub enum AttributeKind {
    /// Floating point 32-bit number.
    Float,
    /// Integer unsigned 8-bit number.
    UnsignedByte,
    /// Integer unsigned 16-bit number.
    UnsignedShort,
    /// Integer unsigned 32-bit number.
    UnsignedInt,
}

/// Vertex attribute definition.
pub struct AttributeDefinition {
    /// Binding point of the vertex attribute.
    pub location: u32,
    /// Vertex attribute kind. See [`AttributeKind`] docs for more info.
    pub kind: AttributeKind,
    /// Component count in the vertex. This could be 1,2,3 or 4.
    pub component_count: usize,
    /// A flag, that defines whether the attribute is normalized or not. Normalized attributes
    /// are always real numbers in `[-1.0; 1.0]` range (or `[0.0; 1.0]` for unsigned attributes).
    pub normalized: bool,
    /// Defines feed rate of the vertex attribute. 0 - means that the attribute will be unique per
    /// vertex, 1 - per each drawn instance, 2 - per two instances and so on.
    pub divisor: u32,
}

impl AttributeKind {
    /// Returns attribute size in bytes.
    pub fn size(self) -> usize {
        match self {
            AttributeKind::Float => size_of::<f32>(),
            AttributeKind::UnsignedByte => size_of::<u8>(),
            AttributeKind::UnsignedShort => size_of::<u16>(),
            AttributeKind::UnsignedInt => size_of::<u32>(),
        }
    }
}

/// Untyped vertex buffer data.
pub struct VertexBufferData<'a> {
    /// Vertex size.
    pub element_size: usize,
    /// Vertex buffer data.
    pub bytes: Option<&'a [u8]>,
}

impl<'a> VertexBufferData<'a> {
    /// Creates a new untyped vertex buffer data from a typed slice. Underlying type must implement
    /// [`Pod`] trait!
    pub fn new<T: Pod>(vertices: Option<&'a [T]>) -> Self {
        Self {
            element_size: size_of::<T>(),
            bytes: vertices.map(|v| array_as_u8_slice(v)),
        }
    }
}

/// Vertex buffer descriptor contains information about vertex buffer layout and content usage as
/// well as the data that will be uploaded to GPU.
pub struct VertexBufferDescriptor<'a> {
    /// Vertex buffer usage. See [`BufferUsage`] docs for more info.
    pub usage: BufferUsage,
    /// Attributes of the vertex buffer.
    pub attributes: &'a [AttributeDefinition],
    /// Data of the vertex buffer. See [`VertexBufferData`] docs for more info.
    pub data: VertexBufferData<'a>,
}

/// Describes elements for the geometry buffer.
pub enum ElementsDescriptor<'a> {
    /// Triangles are formed by a triple of vertex indices.
    Triangles(&'a [TriangleDefinition]),
    /// Lines are formed by a pair of vertex indices.
    Lines(&'a [[u32; 2]]),
    /// Points are just straight vertex indices.
    Points(&'a [u32]),
}

impl ElementsDescriptor<'_> {
    /// Returns element kind of the elements' descriptor.
    pub fn element_kind(&self) -> ElementKind {
        match self {
            ElementsDescriptor::Triangles(_) => ElementKind::Triangle,
            ElementsDescriptor::Lines(_) => ElementKind::Line,
            ElementsDescriptor::Points(_) => ElementKind::Point,
        }
    }
}

/// Descriptor of the geometry buffer. It essentially binds multiple vertex buffers and one element
/// buffer.
pub struct GeometryBufferDescriptor<'a> {
    /// Vertex buffers of the buffer. There must be at least one vertex buffer.
    pub buffers: &'a [VertexBufferDescriptor<'a>],
    /// Usage of the geometry buffer. See [`BufferUsage`] docs for more info.
    pub usage: BufferUsage,
    /// Elements of the geometry buffer.
    pub elements: ElementsDescriptor<'a>,
}

define_as_any_trait!(GpuGeometryBufferAsAny => GpuGeometryBufferTrait);

/// Geometry buffer is a mesh buffer, that could contain multiple vertex buffers and only one
/// element buffer. Element could be either a line or triangle (the most commonly used one).
///
/// ## Examples
///
/// The simplest possible example shows how to create a geometry buffer that has a single triangle:
///
/// ```rust
/// use fyrox_graphics::{
///     buffer::BufferUsage,
///     core::{algebra::Vector3, math::TriangleDefinition},
///     error::FrameworkError,
///     geometry_buffer::{
///         AttributeDefinition, AttributeKind, ElementsDescriptor, GpuGeometryBuffer,
///         GeometryBufferDescriptor, VertexBufferData, VertexBufferDescriptor,
///     },
///     server::GraphicsServer,
/// };
/// use bytemuck::{Pod, Zeroable};
///
/// // Vertex type must implement a bunch of traits, that guarantees that the data will be tightly
/// // packed with expected order of elements.
/// #[derive(Pod, Copy, Clone, Zeroable)]
/// #[repr(C)]
/// struct Vertex {
///     position: Vector3<f32>,
/// }
///
/// fn create_geometry_buffer(
///     server: &dyn GraphicsServer,
/// ) -> Result<GpuGeometryBuffer, FrameworkError> {
///     let vertices = [
///         Vertex {
///             position: Vector3::new(0.0, 0.0, 0.0),
///         },
///         Vertex {
///             position: Vector3::new(0.0, 1.0, 0.0),
///         },
///         Vertex {
///             position: Vector3::new(1.0, 0.0, 0.0),
///         },
///     ];
///
///     let triangles = [TriangleDefinition([0, 1, 2])];
///
///     server.create_geometry_buffer(GeometryBufferDescriptor {
///         buffers: &[VertexBufferDescriptor {
///             usage: BufferUsage::StaticDraw,
///             attributes: &[AttributeDefinition {
///                 location: 0,
///                 kind: AttributeKind::Float,
///                 component_count: 3,
///                 normalized: false,
///                 divisor: 0,
///             }],
///             data: VertexBufferData::new(Some(&vertices)),
///         }],
///         usage: BufferUsage::StaticDraw,
///         elements: ElementsDescriptor::Triangles(&triangles),
///     })
/// }
/// ```
pub trait GpuGeometryBufferTrait: GpuGeometryBufferAsAny {
    /// Write untyped data to a vertex buffer with the given index.
    fn set_buffer_data(&self, buffer: usize, data: &[u8]);

    /// Returns total number of elements in the geometry buffer.
    fn element_count(&self) -> usize;

    /// Writes triangles to the buffer. Each triangle definition contains triangle indices, that
    /// forms the triangle.
    fn set_triangles(&self, triangles: &[TriangleDefinition]);

    /// Writes lines to the buffer. Each pair defines starting and ending vertex index of the line.
    fn set_lines(&self, lines: &[[u32; 2]]);

    /// Writes points to the buffer. Each index in the slice defines vertex index.
    fn set_points(&self, points: &[u32]);
}

impl dyn GpuGeometryBufferTrait {
    /// Writes a typed data to a vertex buffer with the given index. Underlying type must implement
    /// [`Pod`] trait!
    pub fn set_buffer_data_of_type<T: Pod>(&self, buffer: usize, data: &[T]) {
        self.set_buffer_data(buffer, array_as_u8_slice(data))
    }
}

define_shared_wrapper!(GpuGeometryBuffer<dyn GpuGeometryBufferTrait>);
