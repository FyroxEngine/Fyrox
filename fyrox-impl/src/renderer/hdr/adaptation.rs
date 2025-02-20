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

use crate::renderer::{
    framework::{error::FrameworkError, gpu_texture::GpuTexture, server::GraphicsServer},
    hdr::LumBuffer,
};
use std::cell::Cell;

pub struct AdaptationChain {
    lum_framebuffers: [LumBuffer; 2],
    swap: Cell<bool>,
}

pub struct AdaptationContext<'a> {
    pub prev_lum: GpuTexture,
    pub lum_buffer: &'a LumBuffer,
}

impl AdaptationChain {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            lum_framebuffers: [LumBuffer::new(server, 1)?, LumBuffer::new(server, 1)?],
            swap: Cell::new(false),
        })
    }

    pub fn begin(&self) -> AdaptationContext<'_> {
        let out = if self.swap.get() {
            AdaptationContext {
                prev_lum: self.lum_framebuffers[0].framebuffer.color_attachments()[0]
                    .texture
                    .clone(),
                lum_buffer: &self.lum_framebuffers[1],
            }
        } else {
            AdaptationContext {
                prev_lum: self.lum_framebuffers[1].framebuffer.color_attachments()[0]
                    .texture
                    .clone(),
                lum_buffer: &self.lum_framebuffers[0],
            }
        };

        self.swap.set(!self.swap.get());

        out
    }

    pub fn avg_lum_texture(&self) -> &GpuTexture {
        if self.swap.get() {
            &self.lum_framebuffers[0].framebuffer.color_attachments()[0].texture
        } else {
            &self.lum_framebuffers[1].framebuffer.color_attachments()[0].texture
        }
    }
}
