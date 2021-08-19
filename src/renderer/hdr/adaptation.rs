use crate::renderer::{
    framework::{
        error::FrameworkError,
        gpu_program::{GpuProgram, UniformLocation},
        gpu_texture::GpuTexture,
        state::PipelineState,
    },
    hdr::LumBuffer,
};
use std::{cell::RefCell, rc::Rc};

pub struct AdaptationShader {
    program: GpuProgram,
    old_lum_sampler: UniformLocation,
    new_lum_sampler: UniformLocation,
    speed: UniformLocation,
}

impl AdaptationShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/hdr_adaptation_fs.glsl");
        let vertex_source = include_str!("../shaders/flat_vs.glsl");

        let program =
            GpuProgram::from_source(state, "AdaptationShader", vertex_source, fragment_source)?;

        Ok(Self {
            old_lum_sampler: program.uniform_location(state, "oldLumSampler")?,
            new_lum_sampler: program.uniform_location(state, "newLumSampler")?,
            speed: program.uniform_location(state, "speed")?,
            program,
        })
    }
}

pub struct AdaptationChain {
    lum_framebuffers: [LumBuffer; 2],
    swap: bool,
}

pub struct AdaptationContext<'a> {
    prev_lum: Rc<RefCell<GpuTexture>>,
    framebuffer: &'a mut LumBuffer,
}

impl AdaptationChain {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            lum_framebuffers: [LumBuffer::new(state, 1)?, LumBuffer::new(state, 1)?],
            swap: false,
        })
    }

    pub fn begin(&mut self) -> AdaptationContext<'_> {
        let out = if self.swap {
            AdaptationContext {
                prev_lum: self.lum_framebuffers[0].framebuffer.color_attachments()[0]
                    .texture
                    .clone(),
                framebuffer: &mut self.lum_framebuffers[1],
            }
        } else {
            AdaptationContext {
                prev_lum: self.lum_framebuffers[1].framebuffer.color_attachments()[0]
                    .texture
                    .clone(),
                framebuffer: &mut self.lum_framebuffers[0],
            }
        };

        self.swap = !self.swap;

        out
    }
}
