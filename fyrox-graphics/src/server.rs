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
    error::FrameworkError,
    framebuffer::{Attachment, FrameBuffer},
    gpu_program::{GpuProgram, PropertyDefinition},
    gpu_texture::{GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind},
    query::Query,
    read_buffer::AsyncReadBuffer,
    stats::PipelineStatistics,
};
use std::{
    any::Any,
    cell::RefCell,
    fmt::{Display, Formatter},
    rc::{Rc, Weak},
};

pub struct ServerCapabilities {
    pub max_uniform_block_size: usize,
    pub uniform_buffer_offset_alignment: usize,
}

impl Display for ServerCapabilities {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "\tMax Uniform Block Size: {}",
            self.max_uniform_block_size
        )?;
        writeln!(
            f,
            "\tUniform Block Offset Alignment: {}",
            self.uniform_buffer_offset_alignment
        )?;
        Ok(())
    }
}

pub trait GraphicsServer: Any {
    fn create_buffer(
        &self,
        size: usize,
        buffer_kind: BufferKind,
        buffer_usage: BufferUsage,
    ) -> Result<Box<dyn Buffer>, FrameworkError>;
    fn create_texture(
        &self,
        kind: GpuTextureKind,
        pixel_kind: PixelKind,
        min_filter: MinificationFilter,
        mag_filter: MagnificationFilter,
        mip_count: usize,
        data: Option<&[u8]>,
    ) -> Result<Rc<RefCell<dyn GpuTexture>>, FrameworkError>;
    fn create_frame_buffer(
        &self,
        depth_attachment: Option<Attachment>,
        color_attachments: Vec<Attachment>,
    ) -> Result<Box<dyn FrameBuffer>, FrameworkError>;
    fn back_buffer(&self) -> Box<dyn FrameBuffer>;
    fn create_query(&self) -> Result<Box<dyn Query>, FrameworkError>;
    fn create_program(
        &self,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<Box<dyn GpuProgram>, FrameworkError>;
    fn create_program_with_properties(
        &self,
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
        properties: &[PropertyDefinition],
    ) -> Result<Box<dyn GpuProgram>, FrameworkError>;
    fn create_async_read_buffer(
        &self,
        pixel_size: usize,
        pixel_count: usize,
    ) -> Result<Box<dyn AsyncReadBuffer>, FrameworkError>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn weak(self: Rc<Self>) -> Weak<dyn GraphicsServer>;
    fn flush(&self);
    fn finish(&self);
    fn invalidate_resource_bindings_cache(&self);
    fn pipeline_statistics(&self) -> PipelineStatistics;
    fn swap_buffers(&self) -> Result<(), FrameworkError>;
    fn set_frame_size(&self, new_size: (u32, u32));
    fn capabilities(&self) -> ServerCapabilities;
}
