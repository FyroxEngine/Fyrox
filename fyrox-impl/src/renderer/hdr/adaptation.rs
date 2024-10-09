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
    core::sstorage::ImmutableString,
    renderer::{
        framework::{
            error::FrameworkError,
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::GpuTexture,
            server::GraphicsServer,
        },
        hdr::LumBuffer,
    },
};
use std::{cell::RefCell, rc::Rc};

pub struct AdaptationShader {
    pub program: Box<dyn GpuProgram>,
    pub old_lum_sampler: UniformLocation,
    pub new_lum_sampler: UniformLocation,
    pub uniform_buffer_binding: usize,
}

impl AdaptationShader {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_adaptation_fs.glsl");
        let vertex_source = include_str!("../shaders/hdr_adaptation_vs.glsl");

        let program = server.create_program("AdaptationShader", vertex_source, fragment_source)?;

        Ok(Self {
            uniform_buffer_binding: program
                .uniform_block_index(&ImmutableString::new("Uniforms"))?,
            old_lum_sampler: program.uniform_location(&ImmutableString::new("oldLumSampler"))?,
            new_lum_sampler: program.uniform_location(&ImmutableString::new("newLumSampler"))?,
            program,
        })
    }
}

pub struct AdaptationChain {
    lum_framebuffers: [LumBuffer; 2],
    swap: bool,
}

pub struct AdaptationContext<'a> {
    pub prev_lum: Rc<RefCell<dyn GpuTexture>>,
    pub lum_buffer: &'a mut LumBuffer,
}

impl AdaptationChain {
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        Ok(Self {
            lum_framebuffers: [LumBuffer::new(server, 1)?, LumBuffer::new(server, 1)?],
            swap: false,
        })
    }

    pub fn begin(&mut self) -> AdaptationContext<'_> {
        let out = if self.swap {
            AdaptationContext {
                prev_lum: self.lum_framebuffers[0].framebuffer.color_attachments()[0]
                    .texture
                    .clone(),
                lum_buffer: &mut self.lum_framebuffers[1],
            }
        } else {
            AdaptationContext {
                prev_lum: self.lum_framebuffers[1].framebuffer.color_attachments()[0]
                    .texture
                    .clone(),
                lum_buffer: &mut self.lum_framebuffers[0],
            }
        };

        self.swap = !self.swap;

        out
    }

    pub fn avg_lum_texture(&self) -> Rc<RefCell<dyn GpuTexture>> {
        if self.swap {
            self.lum_framebuffers[0].framebuffer.color_attachments()[0]
                .texture
                .clone()
        } else {
            self.lum_framebuffers[1].framebuffer.color_attachments()[0]
                .texture
                .clone()
        }
    }
}
