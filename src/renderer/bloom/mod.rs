use crate::core::sstorage::ImmutableString;
use crate::renderer::framework::geometry_buffer::ElementRange;
use crate::{
    core::{math::Rect, scope_profile},
    renderer::{
        bloom::blur::GaussianBlur,
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, DrawParameters, FrameBuffer},
            geometry_buffer::GeometryBuffer,
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::PipelineState,
        },
        make_viewport_matrix, RenderPassStatistics,
    },
};
use std::{cell::RefCell, rc::Rc};

mod blur;

struct Shader {
    program: GpuProgram,
    world_view_projection_matrix: UniformLocation,
    hdr_sampler: UniformLocation,
}

impl Shader {
    fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/bloom_fs.glsl");
        let vertex_source = include_str!("../shaders/flat_vs.glsl");

        let program =
            GpuProgram::from_source(state, "BloomShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_view_projection_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            hdr_sampler: program.uniform_location(state, &ImmutableString::new("hdrSampler"))?,
            program,
        })
    }
}

pub struct BloomRenderer {
    shader: Shader,
    framebuffer: FrameBuffer,
    blur: GaussianBlur,
    width: usize,
    height: usize,
}

impl BloomRenderer {
    pub fn new(state: &PipelineState, width: usize, height: usize) -> Result<Self, FrameworkError> {
        let frame = {
            let kind = GpuTextureKind::Rectangle { width, height };
            let mut texture = GpuTexture::new(
                state,
                kind,
                PixelKind::RGBA16F,
                MinificationFilter::Nearest,
                MagnificationFilter::Nearest,
                1,
                None,
            )?;
            texture
                .bind_mut(state, 0)
                .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
                .set_wrap(Coordinate::T, WrapMode::ClampToEdge);
            texture
        };

        Ok(Self {
            shader: Shader::new(state)?,
            blur: GaussianBlur::new(state, width, height, PixelKind::RGBA16F)?,
            framebuffer: FrameBuffer::new(
                state,
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(frame)),
                }],
            )?,
            width,
            height,
        })
    }

    fn glow_texture(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub fn result(&self) -> Rc<RefCell<GpuTexture>> {
        self.blur.result()
    }

    pub(crate) fn render(
        &mut self,
        state: &PipelineState,
        quad: &GeometryBuffer,
        hdr_scene_frame: Rc<RefCell<GpuTexture>>,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        scope_profile!();

        let mut stats = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, self.width as i32, self.height as i32);

        let shader = &self.shader;
        stats += self.framebuffer.draw(
            quad,
            state,
            viewport,
            &shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: false,
                blend: None,
                stencil_op: Default::default(),
            },
            ElementRange::Full,
            |mut program_binding| {
                program_binding
                    .set_matrix4(
                        &shader.world_view_projection_matrix,
                        &(make_viewport_matrix(viewport)),
                    )
                    .set_texture(&shader.hdr_sampler, &hdr_scene_frame);
            },
        )?;

        stats += self.blur.render(state, quad, self.glow_texture())?;

        Ok(stats)
    }
}
