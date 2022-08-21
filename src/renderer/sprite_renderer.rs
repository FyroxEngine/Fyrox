use crate::renderer::framework::framebuffer::BlendParameters;
use crate::scene::sprite::Sprite;
use crate::{
    core::{
        math::{Matrix4Ext, Rect},
        scope_profile,
        sstorage::ImmutableString,
    },
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::{CullFace, DrawParameters, FrameBuffer},
            geometry_buffer::{GeometryBuffer, GeometryBufferKind},
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::GpuTexture,
            state::{BlendFactor, BlendFunc, PipelineState},
        },
        RenderPassStatistics, TextureCache,
    },
    scene::{camera::Camera, graph::Graph, mesh::surface::SurfaceData},
};
use std::{cell::RefCell, rc::Rc};

struct SpriteShader {
    program: GpuProgram,
    view_projection_matrix: UniformLocation,
    world_matrix: UniformLocation,
    camera_side_vector: UniformLocation,
    camera_up_vector: UniformLocation,
    color: UniformLocation,
    diffuse_texture: UniformLocation,
    size: UniformLocation,
    rotation: UniformLocation,
}

impl SpriteShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/sprite_fs.glsl");
        let vertex_source = include_str!("shaders/sprite_vs.glsl");
        let program =
            GpuProgram::from_source(state, "SpriteShader", vertex_source, fragment_source)?;
        Ok(Self {
            view_projection_matrix: program
                .uniform_location(state, &ImmutableString::new("viewProjectionMatrix"))?,
            world_matrix: program.uniform_location(state, &ImmutableString::new("worldMatrix"))?,
            camera_side_vector: program
                .uniform_location(state, &ImmutableString::new("cameraSideVector"))?,
            camera_up_vector: program
                .uniform_location(state, &ImmutableString::new("cameraUpVector"))?,
            size: program.uniform_location(state, &ImmutableString::new("size"))?,
            diffuse_texture: program
                .uniform_location(state, &ImmutableString::new("diffuseTexture"))?,
            color: program.uniform_location(state, &ImmutableString::new("color"))?,
            rotation: program.uniform_location(state, &ImmutableString::new("rotation"))?,
            program,
        })
    }
}

pub struct SpriteRenderer {
    shader: SpriteShader,
    collapsed_quad: GeometryBuffer,
}

pub(crate) struct SpriteRenderContext<'a, 'b, 'c> {
    pub state: &'a mut PipelineState,
    pub framebuffer: &'b mut FrameBuffer,
    pub graph: &'c Graph,
    pub camera: &'c Camera,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub viewport: Rect<i32>,
    pub textures: &'a mut TextureCache,
}

impl SpriteRenderer {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let surface = GeometryBuffer::from_surface_data(
            &SurfaceData::make_collapsed_xy_quad(),
            GeometryBufferKind::StaticDraw,
            state,
        );

        Ok(Self {
            shader: SpriteShader::new(state)?,
            collapsed_quad: surface,
        })
    }

    #[must_use]
    pub(crate) fn render(&mut self, args: SpriteRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let SpriteRenderContext {
            state,
            framebuffer,
            graph,
            camera,
            white_dummy,
            viewport,
            textures,
        } = args;

        let initial_view_projection = camera.view_projection_matrix();

        let inv_view = camera.inv_view_matrix().unwrap();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        for sprite in graph.linear_iter().filter_map(|node| {
            if !node.global_visibility() {
                return None;
            }

            node.cast::<Sprite>()
        }) {
            let view_projection = if sprite.depth_offset_factor() != 0.0 {
                let mut projection = camera.projection_matrix();
                projection[14] -= sprite.depth_offset_factor();
                projection * camera.view_matrix()
            } else {
                initial_view_projection
            };

            let diffuse_texture = if let Some(texture) = sprite.texture_ref() {
                if let Some(texture) = textures.get(state, texture) {
                    texture
                } else {
                    white_dummy.clone()
                }
            } else {
                white_dummy.clone()
            };

            statistics += framebuffer.draw(
                &self.collapsed_quad,
                state,
                viewport,
                &self.shader.program,
                &DrawParameters {
                    cull_face: Some(CullFace::Back),
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: None,
                    depth_test: true,
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                        ..Default::default()
                    }),
                    stencil_op: Default::default(),
                },
                |mut program_binding| {
                    program_binding
                        .set_texture(&self.shader.diffuse_texture, &diffuse_texture)
                        .set_matrix4(&self.shader.view_projection_matrix, &view_projection)
                        .set_matrix4(&self.shader.world_matrix, &sprite.global_transform())
                        .set_vector3(&self.shader.camera_up_vector, &camera_up)
                        .set_vector3(&self.shader.camera_side_vector, &camera_side)
                        .set_f32(&self.shader.size, sprite.size())
                        .set_linear_color(&self.shader.color, &sprite.color())
                        .set_f32(&self.shader.rotation, sprite.rotation());
                },
            );
        }

        statistics
    }
}
