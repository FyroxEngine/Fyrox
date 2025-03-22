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
    buffer::GpuBufferTrait,
    core::{color::Color, math::Rect},
    error::FrameworkError,
    framebuffer::ReadTarget,
    framebuffer::{
        Attachment, AttachmentKind, BufferDataUsage, DrawCallStatistics, GpuFrameBuffer,
        GpuFrameBufferTrait, ResourceBindGroup, ResourceBinding,
    },
    geometry_buffer::GpuGeometryBuffer,
    gl::sampler::GlSampler,
    gl::{
        buffer::GlBuffer, geometry_buffer::GlGeometryBuffer, program::GlProgram,
        server::GlGraphicsServer, texture::GlTexture, ToGlConstant,
    },
    gpu_program::GpuProgram,
    gpu_texture::image_2d_size_bytes,
    gpu_texture::{CubeMapFace, GpuTextureKind, GpuTextureTrait, PixelElementKind},
    ColorMask, DrawParameters, ElementRange,
};
use glow::{HasContext, PixelPackData};
use std::rc::Weak;

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
                let texture = depth_attachment
                    .texture
                    .as_any()
                    .downcast_ref::<GlTexture>()
                    .unwrap();
                set_attachment(server, depth_attachment_kind, texture);
            }

            let mut color_buffers = Vec::new();
            for (i, color_attachment) in color_attachments.iter().enumerate() {
                assert_eq!(color_attachment.kind, AttachmentKind::Color);
                let color_attachment_kind = glow::COLOR_ATTACHMENT0 + i as u32;
                let texture = color_attachment
                    .texture
                    .as_any()
                    .downcast_ref::<GlTexture>()
                    .unwrap();
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

impl GpuFrameBufferTrait for GlFrameBuffer {
    fn color_attachments(&self) -> &[Attachment] {
        &self.color_attachments
    }

    fn depth_attachment(&self) -> Option<&Attachment> {
        self.depth_attachment.as_ref()
    }

    fn set_cubemap_face(&self, attachment_index: usize, face: CubeMapFace) {
        let server = self.state.upgrade().unwrap();

        unsafe {
            server.set_framebuffer(self.fbo);

            let attachment = self.color_attachments.get(attachment_index).unwrap();
            let texture = attachment
                .texture
                .as_any()
                .downcast_ref::<GlTexture>()
                .unwrap();
            server.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0 + attachment_index as u32,
                face.into_gl(),
                Some(texture.id()),
                0,
            );
        }
    }

    fn blit_to(
        &self,
        dest: &GpuFrameBuffer,
        src_x0: i32,
        src_y0: i32,
        src_x1: i32,
        src_y1: i32,
        dst_x0: i32,
        dst_y0: i32,
        dst_x1: i32,
        dst_y1: i32,
        copy_color: bool,
        copy_depth: bool,
        copy_stencil: bool,
    ) {
        let server = self.state.upgrade().unwrap();

        let source = self;
        let dest = dest.as_any().downcast_ref::<GlFrameBuffer>().unwrap();

        let mut mask = 0;
        if copy_color {
            mask |= glow::COLOR_BUFFER_BIT;
        }
        if copy_depth {
            mask |= glow::DEPTH_BUFFER_BIT;
        }
        if copy_stencil {
            mask |= glow::STENCIL_BUFFER_BIT;
        }

        unsafe {
            server
                .gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, source.id());
            server
                .gl
                .bind_framebuffer(glow::DRAW_FRAMEBUFFER, dest.id());
            server.gl.blit_framebuffer(
                src_x0,
                src_y0,
                src_x1,
                src_y1,
                dst_x0,
                dst_y0,
                dst_x1,
                dst_y1,
                mask,
                glow::NEAREST,
            );
        }
    }

    fn read_pixels(&self, read_target: ReadTarget) -> Option<Vec<u8>> {
        let server = self.state.upgrade()?;
        server.set_framebuffer(self.id());

        unsafe {
            server
                .gl
                .bind_framebuffer(glow::READ_FRAMEBUFFER, self.id());
        }

        let texture = match read_target {
            ReadTarget::Depth | ReadTarget::Stencil => &self.depth_attachment.as_ref()?.texture,
            ReadTarget::Color(index) => {
                unsafe {
                    server
                        .gl
                        .read_buffer(glow::COLOR_ATTACHMENT0 + index as u32);
                }

                &self.color_attachments.get(index)?.texture
            }
        };

        if let GpuTextureKind::Rectangle { width, height } = texture.kind() {
            let pixel_kind = texture.pixel_kind();
            let pixel_info = pixel_kind.pixel_descriptor();
            let mut buffer = vec![0; image_2d_size_bytes(pixel_kind, width, height)];
            unsafe {
                server.gl.read_pixels(
                    0,
                    0,
                    width as i32,
                    height as i32,
                    pixel_info.format,
                    pixel_info.data_type,
                    PixelPackData::Slice(Some(buffer.as_mut_slice())),
                );
            }
            Some(buffer)
        } else {
            None
        }
    }

    fn clear(
        &self,
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
                    match attachment.texture.pixel_kind().element_kind() {
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
        &self,
        geometry: &GpuGeometryBuffer,
        viewport: Rect<i32>,
        program: &GpuProgram,
        params: &DrawParameters,
        resources: &[ResourceBindGroup],
        element_range: ElementRange,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let server = self.state.upgrade().unwrap();
        let geometry = geometry
            .as_any()
            .downcast_ref::<GlGeometryBuffer>()
            .unwrap();

        pre_draw(self.id(), &server, viewport, program, params, resources);

        let (offset, element_count) = match element_range {
            ElementRange::Full => (0, geometry.element_count.get()),
            ElementRange::Specific { offset, count } => (offset, count),
        };

        let last_element_index = offset + element_count;

        if last_element_index > geometry.element_count.get() {
            Err(FrameworkError::InvalidElementRange {
                start: offset,
                end: last_element_index,
                total: geometry.element_count.get(),
            })
        } else {
            let index_per_element = geometry.element_kind.index_per_element();
            let start_index = offset * index_per_element;
            let index_count = element_count * index_per_element;

            unsafe {
                if index_count > 0 {
                    server.set_vertex_array_object(Some(geometry.vertex_array_object));

                    let indices = (start_index * size_of::<u32>()) as i32;
                    server.gl.draw_elements(
                        geometry.mode(),
                        index_count as i32,
                        glow::UNSIGNED_INT,
                        indices,
                    );
                }
            }

            Ok(DrawCallStatistics {
                triangles: element_count,
            })
        }
    }

    fn draw_instances(
        &self,
        instance_count: usize,
        geometry: &GpuGeometryBuffer,
        viewport: Rect<i32>,
        program: &GpuProgram,
        params: &DrawParameters,
        resources: &[ResourceBindGroup],
        element_range: ElementRange,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        let server = self.state.upgrade().unwrap();
        let geometry = geometry
            .as_any()
            .downcast_ref::<GlGeometryBuffer>()
            .unwrap();

        pre_draw(self.id(), &server, viewport, program, params, resources);

        let (offset, element_count) = match element_range {
            ElementRange::Full => (0, geometry.element_count.get()),
            ElementRange::Specific { offset, count } => (offset, count),
        };

        let last_element_index = offset + element_count;

        if last_element_index > geometry.element_count.get() {
            Err(FrameworkError::InvalidElementRange {
                start: offset,
                end: last_element_index,
                total: geometry.element_count.get(),
            })
        } else {
            let index_per_element = geometry.element_kind.index_per_element();
            let start_index = offset * index_per_element;
            let index_count = geometry.element_count.get() * index_per_element;

            unsafe {
                if index_count > 0 {
                    server.set_vertex_array_object(Some(geometry.vertex_array_object));
                    let indices = (start_index * size_of::<u32>()) as i32;
                    server.gl.draw_elements_instanced(
                        geometry.mode(),
                        index_count as i32,
                        glow::UNSIGNED_INT,
                        indices,
                        instance_count as i32,
                    )
                }
            }

            Ok(DrawCallStatistics {
                triangles: geometry.element_count.get() * instance_count,
            })
        }
    }
}

fn pre_draw(
    fbo: Option<glow::Framebuffer>,
    server: &GlGraphicsServer,
    viewport: Rect<i32>,
    program: &GpuProgram,
    params: &DrawParameters,
    resources: &[ResourceBindGroup],
) {
    server.set_framebuffer(fbo);
    server.set_viewport(viewport);
    server.apply_draw_parameters(params);
    let program = program.as_any().downcast_ref::<GlProgram>().unwrap();
    server.set_program(Some(program.id));

    for bind_group in resources {
        for binding in bind_group.bindings {
            match binding {
                ResourceBinding::Texture {
                    texture,
                    sampler,
                    binding: shader_location,
                } => {
                    let texture = texture.as_any().downcast_ref::<GlTexture>().unwrap();
                    texture.bind(server, *shader_location as u32);
                    let sampler = sampler.as_any().downcast_ref::<GlSampler>().unwrap();
                    unsafe {
                        server
                            .gl
                            .bind_sampler(*shader_location as u32, Some(sampler.id))
                    };
                }
                ResourceBinding::Buffer {
                    buffer,
                    binding,
                    data_usage: data_location,
                } => {
                    let gl_buffer = buffer
                        .as_any()
                        .downcast_ref::<GlBuffer>()
                        .expect("Must be OpenGL buffer");

                    unsafe {
                        let actual_binding = *binding as u32;

                        match data_location {
                            BufferDataUsage::UseSegment { offset, size } => {
                                assert_ne!(*size, 0);
                                server.gl.bind_buffer_range(
                                    gl_buffer.kind().into_gl(),
                                    actual_binding,
                                    Some(gl_buffer.id),
                                    *offset as i32,
                                    *size as i32,
                                );
                            }
                            BufferDataUsage::UseEverything => {
                                server.gl.bind_buffer_base(
                                    gl_buffer.kind().into_gl(),
                                    actual_binding,
                                    Some(gl_buffer.id),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
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
