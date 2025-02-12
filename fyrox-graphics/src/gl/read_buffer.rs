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
    buffer::{BufferKind, BufferUsage, GpuBufferTrait},
    core::{algebra::Vector2, math::Rect},
    error::FrameworkError,
    framebuffer::GpuFrameBufferTrait,
    gl::{buffer::GlBuffer, framebuffer::GlFrameBuffer, server::GlGraphicsServer, ToGlConstant},
    gpu_texture::{image_2d_size_bytes, GpuTextureKind},
    read_buffer::GpuAsyncReadBufferTrait,
};
use glow::{HasContext, PixelPackData};
use std::cell::Cell;
use std::rc::Weak;

#[derive(Copy, Clone)]
struct ReadRequest {
    fence: glow::Fence,
}

pub struct GlAsyncReadBuffer {
    server: Weak<GlGraphicsServer>,
    buffer: GlBuffer,
    request: Cell<Option<ReadRequest>>,
    pixel_count: usize,
    pixel_size: usize,
}

impl GlAsyncReadBuffer {
    pub fn new(
        server: &GlGraphicsServer,
        pixel_size: usize,
        pixel_count: usize,
    ) -> Result<Self, FrameworkError> {
        let size_bytes = pixel_count * pixel_size;
        let buffer = GlBuffer::new(
            server,
            size_bytes,
            BufferKind::PixelRead,
            BufferUsage::StreamRead,
        )?;
        Ok(Self {
            server: server.weak(),
            buffer,
            request: Default::default(),
            pixel_count,
            pixel_size,
        })
    }
}

impl GpuAsyncReadBufferTrait for GlAsyncReadBuffer {
    fn schedule_pixels_transfer(
        &self,
        framebuffer: &dyn GpuFrameBufferTrait,
        color_buffer_index: u32,
        rect: Option<Rect<i32>>,
    ) -> Result<(), FrameworkError> {
        if self.request.get().is_some() {
            return Ok(());
        }

        let server = self.server.upgrade().unwrap();

        let framebuffer = framebuffer
            .as_any()
            .downcast_ref::<GlFrameBuffer>()
            .unwrap();

        let color_attachment = &framebuffer
            .color_attachments()
            .get(color_buffer_index as usize)
            .ok_or_else(|| {
                FrameworkError::Custom(format!(
                    "Framebuffer {:?} does not have {} color attachment!",
                    framebuffer.id(),
                    color_buffer_index
                ))
            })?
            .texture;

        let attachment_pixel_descriptor = color_attachment.pixel_kind().pixel_descriptor();

        let color_attachment_size =
            if let GpuTextureKind::Rectangle { width, height } = color_attachment.kind() {
                Vector2::new(width, height)
            } else {
                return Err(FrameworkError::Custom(
                    "Only rectangular textures can be read from GPU!".to_string(),
                ));
            };

        let actual_size = image_2d_size_bytes(
            color_attachment.pixel_kind(),
            color_attachment_size.x,
            color_attachment_size.y,
        );
        let self_bytes_count = self.pixel_count * self.pixel_size;
        if actual_size != self_bytes_count {
            return Err(FrameworkError::Custom(format!(
                "Pixel buffer size {} does not match the size {} of the color \
                attachment {} of the frame buffer {:?}",
                self_bytes_count,
                actual_size,
                color_buffer_index,
                framebuffer.id()
            )));
        }

        let target_rect = match rect {
            Some(rect) => rect,
            None => Rect::new(
                0,
                0,
                color_attachment_size.x as i32,
                color_attachment_size.y as i32,
            ),
        };

        unsafe {
            let buffer_gl_usage = self.buffer.kind.into_gl();

            server.gl.bind_buffer(buffer_gl_usage, Some(self.buffer.id));

            server
                .gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, framebuffer.id());

            server
                .gl
                .read_buffer(glow::COLOR_ATTACHMENT0 + color_buffer_index);

            server.gl.read_pixels(
                target_rect.position.x,
                target_rect.position.y,
                target_rect.size.x,
                target_rect.size.y,
                attachment_pixel_descriptor.format,
                attachment_pixel_descriptor.data_type,
                PixelPackData::BufferOffset(0),
            );

            server.gl.bind_buffer(buffer_gl_usage, None);

            self.request.set(Some(ReadRequest {
                fence: server
                    .gl
                    .fence_sync(glow::SYNC_GPU_COMMANDS_COMPLETE, 0)
                    .unwrap(),
            }));

            Ok(())
        }
    }

    fn is_request_running(&self) -> bool {
        self.request.get().is_some()
    }

    fn try_read(&self) -> Option<Vec<u8>> {
        let server = self.server.upgrade()?;

        let request = self.request.get()?;

        let mut buffer = vec![0; self.pixel_count * self.pixel_size];

        unsafe {
            // For some reason, glGetSynciv still blocks execution and produces GPU stall, ruining
            // the performance. glClientWaitSync with timeout=0 does not have this issue.
            let fence_state = server.gl.client_wait_sync(request.fence, 0, 0);
            if fence_state != glow::TIMEOUT_EXPIRED && fence_state != glow::WAIT_FAILED {
                self.buffer.read_data(&mut buffer).unwrap();

                server.gl.delete_sync(request.fence);

                self.request.set(None);

                Some(buffer)
            } else {
                None
            }
        }
    }
}
