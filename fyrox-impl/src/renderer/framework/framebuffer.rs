use crate::{
    core::{color::Color, math::Rect, reflect::prelude::*, scope_profile, visitor::prelude::*},
    renderer::framework::{
        error::FrameworkError,
        geometry_buffer::{DrawCallStatistics, ElementRange, GeometryBuffer},
        gpu_program::{GpuProgram, GpuProgramBinding},
        gpu_texture::{CubeMapFace, GpuTexture, GpuTextureKind, PixelElementKind},
        state::{BlendEquation, BlendFunc, ColorMask, PipelineState, StencilFunc, StencilOp},
    },
};
use glow::HasContext;
use serde::{Deserialize, Serialize};
use std::rc::Weak;
use std::{cell::RefCell, rc::Rc};

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Eq)]
pub enum AttachmentKind {
    Color,
    DepthStencil,
    Depth,
}

pub struct Attachment {
    pub kind: AttachmentKind,
    pub texture: Rc<RefCell<GpuTexture>>,
}

pub struct FrameBuffer {
    state: Weak<PipelineState>,
    fbo: Option<glow::Framebuffer>,
    depth_attachment: Option<Attachment>,
    color_attachments: Vec<Attachment>,
}

#[derive(
    Copy, Clone, PartialOrd, PartialEq, Hash, Debug, Serialize, Deserialize, Visit, Eq, Reflect,
)]
#[repr(u32)]
pub enum CullFace {
    Back = glow::BACK,
    Front = glow::FRONT,
}

impl Default for CullFace {
    fn default() -> Self {
        Self::Back
    }
}

#[derive(Serialize, Deserialize, Default, Visit, Debug, PartialEq, Clone, Eq, Reflect)]
pub struct BlendParameters {
    pub func: BlendFunc,
    pub equation: BlendEquation,
}

#[derive(Serialize, Deserialize, Visit, Debug, PartialEq, Clone, Eq, Reflect)]
pub struct DrawParameters {
    pub cull_face: Option<CullFace>,
    pub color_write: ColorMask,
    pub depth_write: bool,
    pub stencil_test: Option<StencilFunc>,
    pub depth_test: bool,
    pub blend: Option<BlendParameters>,
    pub stencil_op: StencilOp,
}

impl Default for DrawParameters {
    fn default() -> Self {
        Self {
            cull_face: Some(CullFace::Back),
            color_write: Default::default(),
            depth_write: true,
            stencil_test: None,
            depth_test: true,
            blend: None,
            stencil_op: Default::default(),
        }
    }
}

unsafe fn set_attachment(state: &PipelineState, gl_attachment_kind: u32, texture: &GpuTexture) {
    match texture.kind() {
        GpuTextureKind::Line { .. } => {
            state.gl.framebuffer_texture(
                glow::FRAMEBUFFER,
                gl_attachment_kind,
                Some(texture.id()),
                0,
            );
        }
        GpuTextureKind::Rectangle { .. } => {
            state.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                gl_attachment_kind,
                glow::TEXTURE_2D,
                Some(texture.id()),
                0,
            );
        }
        GpuTextureKind::Cube { .. } => {
            state.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                gl_attachment_kind,
                glow::TEXTURE_CUBE_MAP_POSITIVE_X,
                Some(texture.id()),
                0,
            );
        }
        GpuTextureKind::Volume { .. } => {
            state.gl.framebuffer_texture_3d(
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

impl FrameBuffer {
    pub fn new(
        state: &PipelineState,
        depth_attachment: Option<Attachment>,
        color_attachments: Vec<Attachment>,
    ) -> Result<Self, FrameworkError> {
        unsafe {
            let fbo = state.gl.create_framebuffer()?;

            state.set_framebuffer(Some(fbo));

            if let Some(depth_attachment) = depth_attachment.as_ref() {
                let depth_attachment_kind = match depth_attachment.kind {
                    AttachmentKind::Color => {
                        panic!("Attempt to use color attachment as depth/stencil!")
                    }
                    AttachmentKind::DepthStencil => glow::DEPTH_STENCIL_ATTACHMENT,
                    AttachmentKind::Depth => glow::DEPTH_ATTACHMENT,
                };
                set_attachment(
                    state,
                    depth_attachment_kind,
                    &depth_attachment.texture.borrow(),
                );
            }

            let mut color_buffers = Vec::new();
            for (i, color_attachment) in color_attachments.iter().enumerate() {
                assert_eq!(color_attachment.kind, AttachmentKind::Color);
                let color_attachment_kind = glow::COLOR_ATTACHMENT0 + i as u32;
                set_attachment(
                    state,
                    color_attachment_kind,
                    &color_attachment.texture.borrow(),
                );
                color_buffers.push(color_attachment_kind);
            }

            if color_buffers.is_empty() {
                state.gl.draw_buffers(&[glow::NONE])
            } else {
                state.gl.draw_buffers(&color_buffers);
            }

            if state.gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                return Err(FrameworkError::FailedToConstructFBO);
            }

            state.set_framebuffer(None);

            Ok(Self {
                state: state.weak(),
                fbo: Some(fbo),
                depth_attachment,
                color_attachments,
            })
        }
    }

    pub fn backbuffer(state: &PipelineState) -> Self {
        Self {
            state: state.weak(),
            fbo: None,
            depth_attachment: None,
            color_attachments: Default::default(),
        }
    }

    pub fn color_attachments(&self) -> &[Attachment] {
        &self.color_attachments
    }

    pub fn depth_attachment(&self) -> Option<&Attachment> {
        self.depth_attachment.as_ref()
    }

    pub fn set_cubemap_face(
        &mut self,
        state: &PipelineState,
        attachment_index: usize,
        face: CubeMapFace,
    ) -> &mut Self {
        unsafe {
            state.set_framebuffer(self.fbo);

            let attachment = self.color_attachments.get(attachment_index).unwrap();
            state.gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0 + attachment_index as u32,
                face.into_gl_value(),
                Some(attachment.texture.borrow().id()),
                0,
            );
        }

        self
    }

    /// None is possible only for back buffer.
    pub fn id(&self) -> Option<glow::Framebuffer> {
        self.fbo
    }

    pub fn clear(
        &mut self,
        state: &PipelineState,
        viewport: Rect<i32>,
        color: Option<Color>,
        depth: Option<f32>,
        stencil: Option<i32>,
    ) {
        scope_profile!();

        state.set_viewport(viewport);
        state.set_framebuffer(self.id());

        unsafe {
            // Special route for default buffer.
            if self.fbo == Default::default() {
                let mut mask = 0;

                if let Some(color) = color {
                    state.set_color_write(ColorMask::default());
                    state.set_clear_color(color);
                    mask |= glow::COLOR_BUFFER_BIT;
                }
                if let Some(depth) = depth {
                    state.set_depth_write(true);
                    state.set_clear_depth(depth);
                    mask |= glow::DEPTH_BUFFER_BIT;
                }
                if let Some(stencil) = stencil {
                    state.set_stencil_mask(0xFFFF_FFFF);
                    state.set_clear_stencil(stencil);
                    mask |= glow::STENCIL_BUFFER_BIT;
                }

                state.gl.clear(mask);
            }

            // Custom routes for specific frame buffer attachments.
            if let Some(depth_stencil) = self.depth_attachment.as_ref() {
                state.set_depth_write(true);
                state.set_stencil_mask(0xFFFF_FFFF);

                match depth_stencil.kind {
                    AttachmentKind::Color => unreachable!("depth cannot be color!"),
                    AttachmentKind::DepthStencil => match (depth, stencil) {
                        (Some(depth), Some(stencil)) => {
                            state.gl.clear_buffer_depth_stencil(
                                glow::DEPTH_STENCIL,
                                0,
                                depth,
                                stencil,
                            );
                        }
                        (Some(depth), None) => {
                            let values = [depth];
                            state.gl.clear_buffer_f32_slice(glow::DEPTH, 0, &values);
                        }
                        (None, Some(stencil)) => {
                            let values = [stencil];
                            state.gl.clear_buffer_i32_slice(glow::STENCIL, 0, &values);
                        }
                        (None, None) => {
                            // Nothing to do
                        }
                    },
                    AttachmentKind::Depth => {
                        if let Some(depth) = depth {
                            let values = [depth];
                            state.gl.clear_buffer_f32_slice(glow::DEPTH, 0, &values);
                        }
                    }
                }
            }

            if let Some(color) = color {
                state.set_color_write(ColorMask::default());

                for (i, attachment) in self.color_attachments.iter().enumerate() {
                    match attachment.texture.borrow().pixel_kind().element_kind() {
                        PixelElementKind::Float | PixelElementKind::NormalizedUnsignedInteger => {
                            let fvalues = color.as_frgba();
                            state.gl.clear_buffer_f32_slice(
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
                            state
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
                            state
                                .gl
                                .clear_buffer_u32_slice(glow::COLOR, i as u32, &values);
                        }
                    }
                }
            }
        }
    }

    pub fn draw<F: FnOnce(GpuProgramBinding<'_, '_>)>(
        &mut self,
        geometry: &GeometryBuffer,
        state: &PipelineState,
        viewport: Rect<i32>,
        program: &GpuProgram,
        params: &DrawParameters,
        element_range: ElementRange,
        apply_uniforms: F,
    ) -> Result<DrawCallStatistics, FrameworkError> {
        scope_profile!();

        pre_draw(self.id(), state, viewport, program, params, apply_uniforms);

        geometry.bind(state).draw(element_range)
    }

    pub fn draw_instances<F: FnOnce(GpuProgramBinding<'_, '_>)>(
        &mut self,
        count: usize,
        geometry: &GeometryBuffer,
        state: &PipelineState,
        viewport: Rect<i32>,
        program: &GpuProgram,
        params: &DrawParameters,
        apply_uniforms: F,
    ) -> DrawCallStatistics {
        scope_profile!();

        pre_draw(self.id(), state, viewport, program, params, apply_uniforms);
        geometry.bind(state).draw_instances(count)
    }
}

fn pre_draw<F: FnOnce(GpuProgramBinding<'_, '_>)>(
    fbo: Option<glow::Framebuffer>,
    state: &PipelineState,
    viewport: Rect<i32>,
    program: &GpuProgram,
    params: &DrawParameters,
    apply_uniforms: F,
) {
    scope_profile!();

    state.set_framebuffer(fbo);
    state.set_viewport(viewport);
    state.apply_draw_parameters(params);

    let program_binding = program.bind(state);
    apply_uniforms(program_binding);
}

impl Drop for FrameBuffer {
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
