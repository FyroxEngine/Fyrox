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

#![warn(missing_docs)]

//! Graphics server is an abstraction layer over various graphics APIs used on different platforms
//! supported by the engine.

use crate::{
    buffer::{Buffer, BufferKind, BufferUsage},
    core::Downcast,
    error::FrameworkError,
    framebuffer::{Attachment, FrameBuffer},
    geometry_buffer::{GeometryBuffer, GeometryBufferDescriptor},
    gpu_program::{GpuProgram, ShaderResourceDefinition},
    gpu_texture::{
        GpuTexture, GpuTextureDescriptor, GpuTextureKind, MagnificationFilter, MinificationFilter,
        PixelKind, WrapMode,
    },
    query::Query,
    read_buffer::AsyncReadBuffer,
    stats::PipelineStatistics,
    PolygonFace, PolygonFillMode,
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

/// Graphics server capabilities.
#[derive(Debug)]
pub struct ServerCapabilities {
    /// The maximum size in basic machine units of a uniform block, which must be at least 16384.
    pub max_uniform_block_size: usize,
    /// The minimum required alignment for uniform buffer sizes and offset. The initial value is 1.
    pub uniform_buffer_offset_alignment: usize,
    /// The maximum, absolute value of the texture level-of-detail bias. The value must be at least
    /// 2.0.
    pub max_lod_bias: f32,
}

/// A shared reference to a graphics server.
pub type SharedGraphicsServer = Rc<dyn GraphicsServer>;

/// Graphics server is an abstraction layer over various graphics APIs used on different platforms
/// supported by the engine. Such abstraction layer tries to provide more or less high-level and
/// unified interface, that can be used to build graphics pipelines quickly and more or less efficiently.
///
/// Low-level GAPI-specific optimizations could be performed using direct access to the underlying API,
/// by downcasting to a specific type.
pub trait GraphicsServer: Downcast {
    /// Creates a GPU buffer with the given size and kind. Usage is a hint to the video driver
    /// that allows to perform some potential performance optimizations.
    fn create_buffer(
        &self,
        size: usize,
        buffer_kind: BufferKind,
        buffer_usage: BufferUsage,
    ) -> Result<Box<dyn Buffer>, FrameworkError>;

    /// Creates a new GPU texture using the given descriptor.
    fn create_texture(
        &self,
        desc: GpuTextureDescriptor,
    ) -> Result<Rc<RefCell<dyn GpuTexture>>, FrameworkError>;

    /// Creates a new frame buffer using the given depth and color attachments. Depth attachment
    /// not exist, but there must be at least one color attachment of a format that supports rendering.
    fn create_frame_buffer(
        &self,
        depth_attachment: Option<Attachment>,
        color_attachments: Vec<Attachment>,
    ) -> Result<Box<dyn FrameBuffer>, FrameworkError>;

    /// Creates a frame buffer that "connected" to the final image that will be displayed to the
    /// screen.
    fn back_buffer(&self) -> Box<dyn FrameBuffer>;

    /// Creates a new GPU query, that can perform asynchronous data fetching from GPU. Usually it
    /// is used to create occlusion queries.
    fn create_query(&self) -> Result<Box<dyn Query>, FrameworkError>;

    /// Creates a new named GPU program using a pair of vertex and fragment shaders. The name could
    /// be used for debugging purposes.
    fn create_program(
        &self,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<Box<dyn GpuProgram>, FrameworkError>;

    /// Almost the same as [`Self::create_program`], but accepts additional array of resource
    /// definitions. The implementation of graphics server will generate proper resource bindings
    /// in the shader code for you.
    fn create_program_with_properties(
        &self,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
        properties: &[ShaderResourceDefinition],
    ) -> Result<Box<dyn GpuProgram>, FrameworkError>;

    /// Creates a new read-back buffer, that can be used to obtain texture data from GPU. It can be
    /// used to read rendering result from GPU to CPU memory and save the result to disk.
    fn create_async_read_buffer(
        &self,
        pixel_size: usize,
        pixel_count: usize,
    ) -> Result<Box<dyn AsyncReadBuffer>, FrameworkError>;

    /// Creates a new geometry buffer, which consists of one or more vertex buffers and only one
    /// element buffer. Geometry buffer could be considered as a complex mesh storage allocated on
    /// GPU.
    fn create_geometry_buffer(
        &self,
        desc: GeometryBufferDescriptor,
    ) -> Result<Box<dyn GeometryBuffer>, FrameworkError>;

    /// Creates a weak reference to the shared graphics server.
    fn weak(self: Rc<Self>) -> Weak<dyn GraphicsServer>;

    /// Sends all scheduled GPU command buffers for execution on GPU without waiting for a certain
    /// threshold.
    fn flush(&self);

    /// Waits until all the scheduled GPU commands are fully executed. This is blocking operation, and
    /// it blocks the current thread until all the commands are fully executed.
    fn finish(&self);

    /// Unbinds the all bound resources from the graphics pipeline.
    fn invalidate_resource_bindings_cache(&self);

    /// Returns GPU pipeline statistics. See [`PipelineStatistics`] for more info.
    fn pipeline_statistics(&self) -> PipelineStatistics;

    /// Swaps the front and back buffers and thus presenting the final image on screen. There could
    /// be more than two buffers, and it is up to the graphics server implementation to choose the
    /// right amount, but it can't be less than two.
    fn swap_buffers(&self) -> Result<(), FrameworkError>;

    /// Notifies the graphics server that the size of the back buffer has changed. It has very limited
    /// use and there are very few platforms (Linux with Wayland mostly) that needs this function to
    /// be called.
    fn set_frame_size(&self, new_size: (u32, u32));

    /// Returns current capabilities of the graphics server. See [`ServerCapabilities`] for more info.
    fn capabilities(&self) -> ServerCapabilities;

    /// Sets current polygon fill mode. See [`PolygonFace`] and [`PolygonFillMode`] docs for more info.
    fn set_polygon_fill_mode(&self, polygon_face: PolygonFace, polygon_fill_mode: PolygonFillMode);

    /// A shortcut for [`Self::create_texture`], that creates a rectangular texture with the given
    /// size and pixel kind.
    fn create_2d_render_target(
        &self,
        pixel_kind: PixelKind,
        width: usize,
        height: usize,
    ) -> Result<Rc<RefCell<dyn GpuTexture>>, FrameworkError> {
        self.create_texture(GpuTextureDescriptor {
            kind: GpuTextureKind::Rectangle { width, height },
            pixel_kind,
            min_filter: MinificationFilter::Nearest,
            mag_filter: MagnificationFilter::Nearest,
            s_wrap_mode: WrapMode::ClampToEdge,
            t_wrap_mode: WrapMode::ClampToEdge,
            r_wrap_mode: WrapMode::ClampToEdge,
            ..Default::default()
        })
    }
}
