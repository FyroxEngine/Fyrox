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

//! Buffer is a type-agnostic data storage located directly in GPU memory. It could be considered
//! as a data block which content is a pile of bytes, whose meaning is defined externally.

use crate::define_shared_wrapper;
use crate::error::FrameworkError;
use bytemuck::Pod;
use fyrox_core::{array_as_u8_slice, array_as_u8_slice_mut, define_as_any_trait};

/// GPU buffer kind.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BufferKind {
    /// Vertex buffer. It is used to supply vertex data (such as positions, normals, texture
    /// coordinates, etc.) to GPU.
    Vertex,
    /// Index buffer. It is used to describe how vertices forms specific types of primitives.
    /// For example a quad could be described by a set of 4 vertices, but 2 triangles which in their
    /// turn consists of 6 indices (0,1,2,0,2,3 vertex indices).
    Index,
    /// Uniform buffer. It is used to supply context-specific data that is needed for rendering.
    /// Usually it contains data that changes from frame-to-frame (world transform, color, etc.).
    Uniform,
    /// Pixel read buffer. It is a special buffer that is used to asynchronously read-back data
    /// from GPU.
    PixelRead,
    /// Pixel write buffer. It is a special buffer that is used to write some data to GPU
    /// asynchronously.
    PixelWrite,
}

/// A hint for video driver that allows it to optimize buffer's content for more efficient use.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BufferUsage {
    /// The buffer contents will be modified once and used at most a few times.
    /// The buffer contents are modified by the application, and used as the source for
    /// drawing and image specification commands.
    StreamDraw,
    /// The buffer contents will be modified once and used at most a few times.
    /// The buffer contents are modified by reading data from the GPU, and used to return that
    /// data when queried by the application.
    StreamRead,
    /// The buffer contents will be modified once and used at most a few times.
    /// The buffer contents are modified by the application, and used as the source for
    /// drawing and image specification commands.
    StreamCopy,
    /// The buffer contents will be modified once and used many times.
    /// The buffer contents are modified by the application, and used as the source for
    /// drawing and image specification commands.
    StaticDraw,
    /// The buffer contents will be modified once and used many times.
    /// The buffer contents are modified by reading data from the GPU, and used to return that
    /// data when queried by the application.
    StaticRead,
    /// The buffer contents will be modified once and used many times.
    /// The buffer contents are modified by reading data from the GPU, and used as the source for
    ///  drawing and image specification commands.
    StaticCopy,
    /// The buffer contents will be modified repeatedly and used many times.
    /// The buffer contents are modified by the application, and used as the source for
    /// drawing and image specification commands.
    DynamicDraw,
    /// The buffer contents will be modified repeatedly and used many times.
    /// The buffer contents are modified by reading data from the GPU, and used to return that
    /// data when queried by the application.
    DynamicRead,
    /// The buffer contents will be modified repeatedly and used many times.
    /// The buffer contents are modified by reading data from the GPU, and used as the source for
    /// drawing and image specification commands.
    DynamicCopy,
}

define_as_any_trait!(GpuBufferAsAny => GpuBufferTrait);

/// Buffer is a type-agnostic data storage located directly in GPU memory. It could be considered
/// as a data block which content is a pile of bytes, whose meaning is defined externally.
///
/// ## Example
///
/// The following example shows how to create a uniform buffer, that could be used for rendering
/// a static object.
///
/// ```rust
/// use fyrox_graphics::{
///     buffer::{GpuBuffer, BufferKind, BufferUsage},
///     core::{algebra::Vector3, color::Color},
///     error::FrameworkError,
///     server::GraphicsServer,
///     uniform::DynamicUniformBuffer,
/// };
///
/// fn create_buffer(server: &dyn GraphicsServer) -> Result<GpuBuffer, FrameworkError> {
///     let uniforms = DynamicUniformBuffer::new()
///         .with(&Vector3::new(1.0, 2.0, 3.0))
///         .with(&Color::WHITE)
///         .with(&123.0f32)
///         .finish();
///
///     let buffer =
///         server.create_buffer(uniforms.len(), BufferKind::Uniform, BufferUsage::StaticDraw)?;
///
///     buffer.write_data(&uniforms)?;
///
///     Ok(buffer)
/// }
/// ```
pub trait GpuBufferTrait: GpuBufferAsAny {
    /// Returns usage kind of the buffer.
    fn usage(&self) -> BufferUsage;
    /// Returns buffer kind.
    fn kind(&self) -> BufferKind;
    /// Returns total size of the buffer in bytes.
    fn size(&self) -> usize;
    /// Writes an arbitrary number of bytes from the given slice.
    fn write_data(&self, data: &[u8]) -> Result<(), FrameworkError>;
    /// Read an arbitrary number of bytes from the buffer (GPU memory) to the given slice. The
    /// amount of the data that will be attempted to read is defined by the length of the given
    /// slice.
    fn read_data(&self, data: &mut [u8]) -> Result<(), FrameworkError>;
}

impl dyn GpuBufferTrait {
    /// Tries to write typed data to the buffer. The data type must implement [`Pod`] trait for
    /// safe usage.
    pub fn write_data_of_type<T: Pod>(&self, data: &[T]) -> Result<(), FrameworkError> {
        let data = array_as_u8_slice(data);
        GpuBufferTrait::write_data(self, data)
    }

    /// Tries to read data from the buffer and convert them to the given type. The data type must
    /// implement [`Pod`] trait for safe usage. The amount of the data that will be attempted to
    /// read is defined by the length of the given slice.
    pub fn read_data_of_type<T: Pod>(&self, data: &mut [T]) -> Result<(), FrameworkError> {
        let data = array_as_u8_slice_mut(data);
        GpuBufferTrait::read_data(self, data)
    }
}

define_shared_wrapper!(GpuBuffer<dyn GpuBufferTrait>);
