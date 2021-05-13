use crate::{
    core::{
        algebra::{Matrix4, Vector3},
        math::Rect,
        scope_profile,
    },
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{Attachment, AttachmentKind, CullFace, DrawParameters, FrameBuffer},
        gpu_program::{GpuProgram, UniformLocation},
        gpu_texture::{
            Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
            PixelKind, WrapMode,
        },
        state::PipelineState,
    },
    renderer::{surface::SurfaceSharedData, GeometryCache},
};
use std::{cell::RefCell, rc::Rc};

struct Shader {
    program: GpuProgram,
    world_view_projection_matrix: UniformLocation,
    input_texture: UniformLocation,
}

impl Shader {
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/blur_fs.glsl");
        let vertex_source = include_str!("shaders/blur_vs.glsl");

        let program = GpuProgram::from_source(state, "FlatShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_view_projection_matrix: program.uniform_location(state, "worldViewProjection")?,
            input_texture: program.uniform_location(state, "inputTexture")?,
            program,
        })
    }
}

pub struct Blur {
    shader: Shader,
    framebuffer: FrameBuffer,
    quad: SurfaceSharedData,
    width: usize,
    height: usize,
}

impl Blur {
    pub fn new(
        state: &mut PipelineState,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        let frame = {
            let kind = GpuTextureKind::Rectangle { width, height };
            let mut texture = GpuTexture::new(
                state,
                kind,
                PixelKind::F32,
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
            framebuffer: FrameBuffer::new(
                state,
                None,
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: Rc::new(RefCell::new(frame)),
                }],
            )?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
            width,
            height,
        })
    }

    pub fn result(&self) -> Rc<RefCell<GpuTexture>> {
        self.framebuffer.color_attachments()[0].texture.clone()
    }

    pub(in crate) fn render(
        &mut self,
        state: &mut PipelineState,
        geom_cache: &mut GeometryCache,
        input: Rc<RefCell<GpuTexture>>,
    ) {
        scope_profile!();

        let viewport = Rect::new(0, 0, self.width as i32, self.height as i32);

        let shader = &self.shader;
        self.framebuffer.draw(
            geom_cache.get(state, &self.quad),
            state,
            viewport,
            &shader.program,
            &DrawParameters {
                cull_face: CullFace::Back,
                culling: false,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: false,
                depth_test: false,
                blend: false,
            },
            |program_binding| {
                program_binding
                    .set_matrix4(
                        &shader.world_view_projection_matrix,
                        &(Matrix4::new_orthographic(
                            0.0,
                            viewport.w() as f32,
                            viewport.h() as f32,
                            0.0,
                            -1.0,
                            1.0,
                        ) * Matrix4::new_nonuniform_scaling(&Vector3::new(
                            viewport.w() as f32,
                            viewport.h() as f32,
                            0.0,
                        ))),
                    )
                    .set_sampler(&shader.input_texture, 0, &input);
            },
        );
    }
}
