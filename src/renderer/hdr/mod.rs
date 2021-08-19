use crate::renderer::hdr::adaptation::AdaptationChain;
use crate::{
    core::{
        algebra::{Matrix4, Vector3},
        color::Color,
        math::Rect,
    },
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, FrameBuffer},
            gpu_texture::{
                GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
            },
            state::PipelineState,
        },
        hdr::{
            adaptation::AdaptationShader, downscale::DownscaleShader, luminance::LuminanceShader,
        },
    },
};
use std::{cell::RefCell, rc::Rc};

mod adaptation;
mod downscale;
mod luminance;

struct LumBuffer {
    framebuffer: FrameBuffer,
    size: usize,
}

impl LumBuffer {
    fn new(state: &mut PipelineState, size: usize) -> Result<Self, FrameworkError> {
        let texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle {
                width: size,
                height: size,
            },
            PixelKind::F32,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        Ok(Self {
            framebuffer: FrameBuffer::new(
                state,
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(texture)),
                }],
            )?,
            size,
        })
    }

    fn clear(&mut self, state: &mut PipelineState) {
        self.framebuffer.clear(
            state,
            Rect::new(0, 0, self.size as i32, self.size as i32),
            Some(Color::BLACK),
            None,
            None,
        );
    }

    fn matrix(&self) -> Matrix4<f32> {
        Matrix4::new_orthographic(0.0, self.size as f32, self.size as f32, 0.0, -1.0, 1.0)
            * Matrix4::new_nonuniform_scaling(&Vector3::new(
                self.size as f32,
                self.size as f32,
                0.0,
            ))
    }
}

pub struct HighDynamicRangeRenderer {
    adaptation_chain: AdaptationChain,
    downscale_chain: [LumBuffer; 6],
    frame_luminance: LumBuffer,
    adaptation_shader: AdaptationShader,
    luminance_shader: LuminanceShader,
    downscale_shader: DownscaleShader,
}

impl HighDynamicRangeRenderer {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            frame_luminance: LumBuffer::new(state, 64)?,
            downscale_chain: [
                LumBuffer::new(state, 32)?,
                LumBuffer::new(state, 16)?,
                LumBuffer::new(state, 8)?,
                LumBuffer::new(state, 4)?,
                LumBuffer::new(state, 2)?,
                LumBuffer::new(state, 1)?,
            ],
            adaptation_chain: AdaptationChain::new(state)?,
            adaptation_shader: AdaptationShader::new(state)?,
            luminance_shader: LuminanceShader::new(state)?,
            downscale_shader: DownscaleShader::new(state)?,
        })
    }

    pub fn render(&mut self, state: &mut PipelineState, scene_frame: Rc<RefCell<GpuTexture>>) {
        self.frame_luminance.clear(state);
        let frame_matrix = self.frame_luminance.matrix();
    }
}
