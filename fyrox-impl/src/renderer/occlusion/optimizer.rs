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
    core::{color::Color, math::Rect, ImmutableString},
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, FrameBuffer},
            geometry_buffer::GeometryBuffer,
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{
                GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
            },
            pixel_buffer::PixelBuffer,
            state::GlGraphicsServer,
            ColorMask, DrawParameters, ElementRange,
        },
        make_viewport_matrix,
    },
};
use fyrox_graphics::state::GraphicsServer;
use std::{cell::RefCell, rc::Rc};

struct VisibilityOptimizerShader {
    program: GpuProgram,
    view_projection: UniformLocation,
    tile_size: UniformLocation,
    visibility_buffer: UniformLocation,
}

impl VisibilityOptimizerShader {
    fn new(server: &GlGraphicsServer) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/visibility_optimizer_fs.glsl");
        let vertex_source = include_str!("../shaders/visibility_optimizer_vs.glsl");
        let program = GpuProgram::from_source(
            server,
            "VisibilityOptimizerShader",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            view_projection: program
                .uniform_location(server, &ImmutableString::new("viewProjection"))?,
            tile_size: program.uniform_location(server, &ImmutableString::new("tileSize"))?,
            visibility_buffer: program
                .uniform_location(server, &ImmutableString::new("visibilityBuffer"))?,
            program,
        })
    }
}

pub struct VisibilityBufferOptimizer {
    framebuffer: Box<dyn FrameBuffer>,
    pixel_buffer: PixelBuffer<u32>,
    shader: VisibilityOptimizerShader,
    w_tiles: usize,
    h_tiles: usize,
}

impl VisibilityBufferOptimizer {
    pub fn new(
        server: &GlGraphicsServer,
        w_tiles: usize,
        h_tiles: usize,
    ) -> Result<Self, FrameworkError> {
        let optimized_visibility_buffer = server.create_texture(
            GpuTextureKind::Rectangle {
                width: w_tiles,
                height: h_tiles,
            },
            PixelKind::R32UI,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;

        Ok(Self {
            framebuffer: server.create_frame_buffer(
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: optimized_visibility_buffer,
                }],
            )?,
            pixel_buffer: PixelBuffer::new(server, w_tiles * h_tiles)?,
            shader: VisibilityOptimizerShader::new(server)?,
            w_tiles,
            h_tiles,
        })
    }

    pub fn is_reading_from_gpu(&self) -> bool {
        self.pixel_buffer.is_request_running()
    }

    pub fn read_visibility_mask(&mut self, server: &GlGraphicsServer) -> Option<Vec<u32>> {
        self.pixel_buffer.try_read(server)
    }

    pub fn optimize(
        &mut self,
        server: &GlGraphicsServer,
        visibility_buffer: &Rc<RefCell<dyn GpuTexture>>,
        unit_quad: &GeometryBuffer,
        tile_size: i32,
    ) -> Result<(), FrameworkError> {
        let viewport = Rect::new(0, 0, self.w_tiles as i32, self.h_tiles as i32);

        self.framebuffer
            .clear(viewport, Some(Color::TRANSPARENT), None, None);

        let matrix = make_viewport_matrix(viewport);

        self.framebuffer.draw(
            unit_quad,
            viewport,
            &self.shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: ColorMask::all(true),
                depth_write: false,
                stencil_test: None,
                depth_test: None,
                blend: None,
                stencil_op: Default::default(),
                scissor_box: None,
            },
            &[], // TODO
            ElementRange::Full,
            &mut |mut program_binding| {
                program_binding
                    .set_matrix4(&self.shader.view_projection, &matrix)
                    .set_texture(&self.shader.visibility_buffer, visibility_buffer)
                    .set_i32(&self.shader.tile_size, tile_size);
            },
        )?;

        self.pixel_buffer
            .schedule_pixels_transfer(server, &*self.framebuffer, 0, None)?;

        Ok(())
    }
}
