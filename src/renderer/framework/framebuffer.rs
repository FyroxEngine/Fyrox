use crate::{
    core::{color::Color, math::Rect, scope_profile},
    renderer::{
        error::RendererError,
        framework::{
            geometry_buffer::{DrawCallStatistics, GeometryBuffer},
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::{CubeMapFace, GpuTexture, GpuTextureKind},
            state::{ColorMask, PipelineState},
        },
    },
};
use glow::HasContext;
use std::{cell::RefCell, rc::Rc};

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug)]
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
    state: *mut PipelineState,
    fbo: glow::Framebuffer,
    depth_attachment: Option<Attachment>,
    color_attachments: Vec<Attachment>,
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Hash, Debug)]
pub enum CullFace {
    Back,
    Front,
}

impl CullFace {
    pub fn into_gl_value(self) -> u32 {
        match self {
            Self::Front => glow::FRONT,
            Self::Back => glow::BACK,
        }
    }
}

pub struct DrawParameters {
    pub cull_face: CullFace,
    pub culling: bool,
    pub color_write: ColorMask,
    pub depth_write: bool,
    pub stencil_test: bool,
    pub depth_test: bool,
    pub blend: bool,
}

impl Default for DrawParameters {
    fn default() -> Self {
        Self {
            cull_face: CullFace::Back,
            culling: true,
            color_write: Default::default(),
            depth_write: true,
            stencil_test: false,
            depth_test: true,
            blend: false,
        }
    }
}

unsafe fn set_attachment(state: &mut PipelineState, gl_attachment_kind: u32, texture: &GpuTexture) {
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
        state: &mut PipelineState,
        depth_attachment: Option<Attachment>,
        color_attachments: Vec<Attachment>,
    ) -> Result<Self, RendererError> {
        unsafe {
            let fbo = state.gl.create_framebuffer()?;

            state.set_framebuffer(fbo);

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
                state.gl.draw_buffer(glow::NONE)
            } else {
                state.gl.draw_buffers(&color_buffers);
            }

            if state.gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                return Err(RendererError::FailedToConstructFBO);
            }

            state.set_framebuffer(0);

            Ok(Self {
                state,
                fbo,
                depth_attachment,
                color_attachments,
            })
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
        state: &mut PipelineState,
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
}

fn pre_draw(
    fbo: glow::Framebuffer,
    state: &mut PipelineState,
    viewport: Rect<i32>,
    program: &GpuProgram,
    params: &DrawParameters,
    uniforms: &[(UniformLocation, UniformValue<'_>)],
) {
    scope_profile!();

    state.set_framebuffer(fbo);
    state.set_viewport(viewport);
    state.apply_draw_parameters(params);

    program.bind(state);
    for (location, value) in uniforms {
        program.set_uniform(state, *location, value)
    }
}

pub struct DrawPartContext<'a, 'b, 'c, 'd> {
    pub state: &'a mut PipelineState,
    pub viewport: Rect<i32>,
    pub geometry: &'a mut GeometryBuffer,
    pub program: &'b mut GpuProgram,
    pub params: DrawParameters,
    pub uniforms: &'c [(UniformLocation, UniformValue<'d>)],
    pub offset: usize,
    pub count: usize,
}

pub trait FrameBufferTrait {
    fn id(&self) -> u32;

    fn clear(
        &mut self,
        state: &mut PipelineState,
        viewport: Rect<i32>,
        color: Option<Color>,
        depth: Option<f32>,
        stencil: Option<i32>,
    ) {
        scope_profile!();

        let mut mask = 0;

        state.set_viewport(viewport);
        state.set_framebuffer(self.id());

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

        unsafe {
            state.gl.clear(mask);
        }
    }

    fn draw(
        &mut self,
        geometry: &GeometryBuffer,
        state: &mut PipelineState,
        viewport: Rect<i32>,
        program: &GpuProgram,
        params: &DrawParameters,
        uniforms: &[(UniformLocation, UniformValue<'_>)],
    ) -> DrawCallStatistics {
        scope_profile!();

        pre_draw(self.id(), state, viewport, program, params, uniforms);
        geometry.bind(state).draw()
    }

    fn draw_instances(
        &mut self,
        count: usize,
        geometry: &GeometryBuffer,
        state: &mut PipelineState,
        viewport: Rect<i32>,
        program: &GpuProgram,
        params: &DrawParameters,
        uniforms: &[(UniformLocation, UniformValue<'_>)],
    ) -> DrawCallStatistics {
        scope_profile!();

        pre_draw(self.id(), state, viewport, program, params, uniforms);
        geometry.bind(state).draw_instances(count)
    }

    fn draw_part(&mut self, args: DrawPartContext) -> Result<DrawCallStatistics, RendererError> {
        scope_profile!();

        pre_draw(
            self.id(),
            args.state,
            args.viewport,
            args.program,
            &args.params,
            args.uniforms,
        );
        args.geometry
            .bind(args.state)
            .draw_part(args.offset, args.count)
    }
}

impl FrameBufferTrait for FrameBuffer {
    fn id(&self) -> u32 {
        self.fbo
    }
}

pub struct BackBuffer;

impl FrameBufferTrait for BackBuffer {
    fn id(&self) -> u32 {
        0
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        unsafe {
            (*self.state).gl.delete_framebuffer(self.fbo);
        }
    }
}
