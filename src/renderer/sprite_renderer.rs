use crate::{
    core::{math::Matrix4Ext, math::Rect, scope_profile},
    renderer::{surface::SurfaceSharedData, GeometryCache, RenderPassStatistics, TextureCache},
    rendering_framework::{
        error::FrameworkError,
        framebuffer::{CullFace, DrawParameters, FrameBuffer, FrameBufferTrait},
        gpu_program::{GpuProgram, UniformLocation, UniformValue},
        gpu_texture::GpuTexture,
        state::PipelineState,
    },
    scene::{camera::Camera, graph::Graph, node::Node},
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
        let program = GpuProgram::from_source(state, "FlatShader", vertex_source, fragment_source)?;
        Ok(Self {
            view_projection_matrix: program.uniform_location(state, "viewProjectionMatrix")?,
            world_matrix: program.uniform_location(state, "worldMatrix")?,
            camera_side_vector: program.uniform_location(state, "cameraSideVector")?,
            camera_up_vector: program.uniform_location(state, "cameraUpVector")?,
            size: program.uniform_location(state, "size")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            color: program.uniform_location(state, "color")?,
            rotation: program.uniform_location(state, "rotation")?,
            program,
        })
    }
}

pub struct SpriteRenderer {
    shader: SpriteShader,
    surface: SurfaceSharedData,
}

pub(in crate) struct SpriteRenderContext<'a, 'b, 'c> {
    pub state: &'a mut PipelineState,
    pub framebuffer: &'b mut FrameBuffer,
    pub graph: &'c Graph,
    pub camera: &'c Camera,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub viewport: Rect<i32>,
    pub textures: &'a mut TextureCache,
    pub geom_map: &'a mut GeometryCache,
}

impl SpriteRenderer {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let surface = SurfaceSharedData::make_collapsed_xy_quad();

        Ok(Self {
            shader: SpriteShader::new(state)?,
            surface,
        })
    }

    #[must_use]
    pub(in crate) fn render(&mut self, args: SpriteRenderContext) -> RenderPassStatistics {
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
            geom_map,
        } = args;

        state.set_blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

        let initial_view_projection = camera.view_projection_matrix();

        let inv_view = camera.inv_view_matrix().unwrap();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        for sprite in graph.linear_iter().filter_map(|node| {
            if !node.global_visibility() {
                return None;
            }

            if let Node::Sprite(sprite) = node {
                Some(sprite)
            } else {
                None
            }
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
                geom_map.get(state, &self.surface),
                state,
                viewport,
                &self.shader.program,
                &DrawParameters {
                    cull_face: CullFace::Back,
                    culling: true,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: false,
                    depth_test: true,
                    blend: true,
                },
                &[
                    (
                        self.shader.diffuse_texture.clone(),
                        UniformValue::Sampler {
                            index: 0,
                            texture: diffuse_texture,
                        },
                    ),
                    (
                        self.shader.view_projection_matrix.clone(),
                        UniformValue::Matrix4(&view_projection),
                    ),
                    (
                        self.shader.world_matrix.clone(),
                        UniformValue::Matrix4(&sprite.global_transform()),
                    ),
                    (
                        self.shader.camera_up_vector.clone(),
                        UniformValue::Vector3(&camera_up),
                    ),
                    (
                        self.shader.camera_side_vector.clone(),
                        UniformValue::Vector3(&camera_side),
                    ),
                    (self.shader.size.clone(), UniformValue::Float(sprite.size())),
                    (
                        self.shader.color.clone(),
                        UniformValue::Color(sprite.color()),
                    ),
                    (
                        self.shader.rotation.clone(),
                        UniformValue::Float(sprite.rotation()),
                    ),
                ],
            );
        }

        statistics
    }
}
