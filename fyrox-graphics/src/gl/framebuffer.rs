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
    buffer::{Buffer, BufferKind},
    core::{color::Color, math::Rect},
    error::FrameworkError,
    framebuffer::{Attachment, AttachmentKind, FrameBuffer, ResourceBinding},
    geometry_buffer::{DrawCallStatistics, GeometryBuffer},
    gl::{buffer::GlBuffer, texture::GlTexture},
    gpu_program::{GpuProgram, GpuProgramBinding},
    gpu_texture::{CubeMapFace, GpuTexture, GpuTextureKind, PixelElementKind},
    state::{GlGraphicsServer, ToGlConstant},
    ColorMask, DrawParameters, ElementRange,
};
use glow::HasContext;
use std::{any::Any, rc::Weak};

pub struct GlFrameBuffer {
    state: Weak<GlGraphicsServer>,
    fbo: Option<glow::Framebuffer>,
    depth_attachment: Option<Attachment>,
    color_attachments: Vec<Attachment>,
}

unsafe fn set_attachment(server: &GlGraphicsServer, gl_attachment_kind: u32, texture: &GlTexture) {
    match texture.kind() {
        GpuTextureKind::Line { .. } => {
            server.gl.framebuffer_texture(
                glow::FRAMEBUFFER,
                gl_attachment_kind,
                Some(texture.id()),
                0,
            );
        }
        GpuTextureKind::Rectangle { .. } => {
            server.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                gl_attachment_kind,
                glow::TEXTURE_2D,
                Some(texture.id()),
                0,
            );
        }
        GpuTextureKind::Cube { .. } => {
            server.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                gl_attachment_kind,
                glow::TEXTURE_CUBE_MAP_POSITIVE_X,
                Some(texture.id()),
                0,
            );
        }
        GpuTextureKind::Volume { .. } => {
            server.gl.framebuffer_texture_3d(
                glow::FRAMEBUFFER,
                gl_attachment_kind,
                glow::TEXTURE_3D,
                Some(texture.id()),
                0,
                0,
            );
        }
    }
}

impl GlFrameBuffer {
    pub fn new(
        server: &GlGraphicsServer,
        depth_attachment: Option<Attachment>,
        color_attachments: Vec<Attachment>,
    ) -> Result<Self, FrameworkError> {
        unsafe {
            let fbo = server.gl.create_framebuffer()?;

            server.set_framebuffer(Some(fbo));

            if let Some(depth_attachment) = depth_attachment.as_ref() {
                let depth_attachment_kind = match depth_attachment.kind {
                    AttachmentKind::Color => {
                        panic!("Attempt to use color attachment as depth/stencil!")
                    }
                    AttachmentKind::DepthStencil => glow::DEPTH_STENCIL_ATTACHMENT,
                    AttachmentKind::Depth => glow::DEPTH_ATTACHMENT,
                };
                let guard = depth_attachment.texture.borrow();
                let texture = guard.as_any().downcast_ref::<GlTexture>().unwrap();
                set_attachment(server, depth_attachment_kind, texture);
            }

            let mut color_buffers = Vec::new();
            for (i, color_attachment) in color_attachments.iter().enumerate() {
                assert_eq!(color_attachment.kind, AttachmentKind::Color);
                let color_attachment_kind = glow::COLOR_ATTACHMENT0 + i as u32;
                let guard = color_attachment.texture.borrow();
                let texture = guard.as_any().downcast_ref::<GlTexture>().unwrap();
                set_attachment(server, color_attachment_kind, texture);
                color_buffers.push(color_attachment_kind);
            }

            if color_buffers.is_empty() {
                server.gl.draw_buffers(&[glow::NONE])
            } else {
                server.gl.draw_buffers(&color_buffers);
            }

            if server.gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                return Err(FrameworkError::FailedToConstructFBO);
            }

            server.set_framebuffer(None);

            Ok(Self {
                state: server.weak(),
                fbo: Some(fbo),
                depth_attachment,
                color_attachments,
            })
        }
    }

    pub fn backbuffer(server: &GlGraphicsServer) -> Self {
        Self {
            state: server.weak(),
            fbo: None,
            depth_attachment: None,
            color_attachments: Default::default(),
        }
    }

    /// None is possible only for back buffer.
    pub fn id(&self) -> Option<glow::Framebuffer> {
        self.fbo
    }
}

impl FrameBuffer for GlFrameBuffer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn color_attachments(&self) -> &[Attachment] {
        &self.color_attachments
    }

    fn depth_attachment(&self) -> Option<&Attachment> {
        self.depth_attachment.as_ref()
    }

    fn set_cubemap_face(&mut self, attachment_index: usize, face: CubeMapFace) {
        let server = self.state.upgrade().unwrap();

        unsafe {
            server.set_framebuffer(self.fbo);

            let attachment = self.color_attachments.get(attachment_index).unwrap();
            let guard = attachment.texture.borrow();
            let texture = guard.as_any().downcast_ref::<GlTexture>().unwrap();
            server.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0 + attachment_index as u32,
                face.into_gl(),
                Some(texture.id()),
                0,
            );
        }
    }

    fn clear(
        &mut self,
        viewport: Rect<i32>,
        color: Option<Color>,
        depth: Option<f32>,
        stencil: Option<i32>,
    ) {
        let server = self.state.upgrade().unwrap();

        server.set_scissor_test(false);
        server.set_viewport(viewport);
        server.set_framebuffer(self.id());

        unsafe {
            // Special route for default buffer.
            if self.fbo == Default::default() {
                let mut mask = 0;

                if let Some(color) = color {
                    server.set_color_write(ColorMask::default());
                    server.set_clear_color(color);
                    mask |= glow::COLOR_BUFFER_BIT;
                }
                if let Some(depth) = depth {
                    server.set_depth_write(true);
                    server.set_clear_depth(depth);
                    mask |= glow::DEPTH_BUFFER_BIT;
                }
                if let Some(stencil) = stencil {
                    server.set_stencil_mask(0xFFFF_FFFF);
                    server.set_clear_stencil(stencil);
                    mask |= glow::STENCIL_BUFFER_BIT;
                }

                server.gl.clear(mask);
            }

            // Custom routes for specific frame buffer attachments.
            if let Some(depth_stencil) = self.depth_attachment.as_ref() {
                server.set_depth_write(true);
                server.set_stencil_mask(0xFFFF_FFFF);

                match depth_stencil.kind {
                    AttachmentKind::Color => unreachable!("depth cannot be color!"),
                    AttachmentKind::DepthStencil => match (depth, stencil) {
                        (Some(depth), Some(stencil)) => {
                            server.gl.clear_buffer_depth_stencil(
                                glow::DEPTH_STENCIL,
                                0,
                                depth,
                                stencil,
                            );
                        }
                        (Some(depth), None) => {
                            let values = [depth];
                            server.gl.clear_buffer_f32_slice(glow::DEPTH, 0, &values);
                        }
                        (None, Some(stencil)) => {
                            let values = [stencil];
                            server.gl.clear_buffer_i32_slice(glow::STENCIL, 0, &values);
                        }
                        (None, None) => {
                            // Nothing to do
                        }
                    },
                    AttachmentKind::Depth => {
                        if let Some(depth) = depth {
                            let values = [depth];
                            server.gl.clear_buffer_f32_slice(glow::DEPTH, 0, &values);
                        }
                    }
                }
            }

            if let Some(color) = color {
                server.set_color_write(ColorMask::default());

                for (i, attachment) in self.color_attachments.iter().enumerate() {
                    match attachment.texture.borrow().pixel_kind().element_kind() {
                        PixelElementKind::Float | PixelElementKind::NormalizedUnsignedInteger => {
                            let fvalues = color.as_frgba();
                            server.gl.clear_buffer_f32_slice(
                                glow::COLOR,
                                i as u32,
                                &fvalues.data.0[0],
                            )
                        }
                        PixelElementKind::Integer => {
                            let values = [
                                color.r as i32,
                                color.g as i32,
                                color.b as i32,
                                color.a as i32,
                            ];
                            server
                                .gl
                                .clear_buffer_i32_slice(glow::COLOR, i as u32, &values);
                        }
                        PixelElementKind::UnsignedInteger => {
                            let values = [
                                color.r as u32,
                                color.g as u32,
                                color.b as u32,
                                color.a as u32,
                            ];
                            server
                                .gl
                                .clear_buffer_u32_slice(glow::COLOR, i as u32, &values);
                        }
                    }
                }
            }
        }
    }

    fn draw(
        &mut self,
        geometry: &GeometryBuffer,
        viewport: Rect<i32>,
        program: &GpuProgram,
        params: &DrawParameters,
        resources: &[ResourceBinding],
        element_range: ElementRange,
        apply_uniforms: &mut dyn FnMut(GpuProgramBinding<'_, '_>),
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let server = self.state.upgrade().unwrap();

        pre_draw(
            self.id(),
            &server,
            viewport,
            program,
            params,
            resources,
            apply_uniforms,
        );

        geometry.bind(&server).draw(element_range)
    }

    fn draw_instances(
        &mut self,
        count: usize,
        geometry: &GeometryBuffer,
        viewport: Rect<i32>,
        program: &GpuProgram,
        params: &DrawParameters,
        resources: &[ResourceBinding],
        apply_uniforms: &mut dyn FnMut(GpuProgramBinding<'_, '_>),
    ) -> DrawCallStatistics {
        let server = self.state.upgrade().unwrap();

        pre_draw(
            self.id(),
            &server,
            viewport,
            program,
            params,
            resources,
            apply_uniforms,
        );

        geometry.bind(&server).draw_instances(count)
    }
}

fn pre_draw(
    fbo: Option<glow::Framebuffer>,
    server: &GlGraphicsServer,
    viewport: Rect<i32>,
    program: &GpuProgram,
    params: &DrawParameters,
    resources: &[ResourceBinding],
    apply_uniforms: &mut dyn FnMut(GpuProgramBinding<'_, '_>),
) {
    server.set_framebuffer(fbo);
    server.set_viewport(viewport);
    server.apply_draw_parameters(params);

    let program_binding = program.bind(server);

    let mut texture_unit = 0;
    let mut buffer_binding = 0;
    for binding in resources {
        match binding {
            ResourceBinding::Texture {
                texture,
                shader_location,
            } => {
                let texture = texture.as_any().downcast_ref::<GlTexture>().unwrap();
                unsafe {
                    server
                        .gl
                        .uniform_1_i32(Some(&shader_location.id), texture_unit)
                };
                texture.bind(server, texture_unit as u32);
                texture_unit += 1;
            }
            ResourceBinding::Buffer {
                buffer,
                shader_location,
            } => {
                let gl_buffer = buffer
                    .as_any()
                    .downcast_ref::<GlBuffer>()
                    .expect("Must be OpenGL buffer");

                unsafe {
                    server.gl.bind_buffer_base(
                        gl_buffer.kind().into_gl(),
                        buffer_binding,
                        Some(gl_buffer.id),
                    );

                    match gl_buffer.kind() {
                        BufferKind::Uniform => server.gl.uniform_block_binding(
                            program_binding.program.id,
                            *shader_location,
                            buffer_binding,
                        ),
                        BufferKind::Vertex
                        | BufferKind::Index
                        | BufferKind::PixelRead
                        | BufferKind::PixelWrite => {}
                    }

                    buffer_binding += 1;
                }
            }
        }
    }

    apply_uniforms(program_binding);
}

impl Drop for GlFrameBuffer {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            unsafe {
                if let Some(id) = self.fbo {
                    state.gl.delete_framebuffer(id);
                }
            }
        }
    }
}
