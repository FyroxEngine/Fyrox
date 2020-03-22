use crate::{
    scene::{
        node::Node,
        base::AsBase,
        graph::Graph,
        camera::Camera,
    },
    core::{
        math::Rect,
    },
    renderer::{
        TextureCache,
        GeometryCache,
        surface::SurfaceSharedData,
        error::RendererError,
        framework::{
            gpu_texture::GpuTexture,
            gl,
            gpu_program::{
                UniformValue,
                GpuProgram,
                UniformLocation,
            },
            framebuffer::{
                FrameBuffer,
                DrawParameters,
                CullFace,
                FrameBufferTrait,
            },
            state::State
        },
        RenderPassStatistics,
    },
};
use std::{
    rc::Rc,
    cell::RefCell,
};

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
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/sprite_fs.glsl");
        let vertex_source = include_str!("shaders/sprite_vs.glsl");
        let mut program = GpuProgram::from_source("FlatShader", vertex_source, fragment_source)?;
        Ok(Self {
            view_projection_matrix: program.uniform_location("viewProjectionMatrix")?,
            world_matrix: program.uniform_location("worldMatrix")?,
            camera_side_vector: program.uniform_location("cameraSideVector")?,
            camera_up_vector: program.uniform_location("cameraUpVector")?,
            size: program.uniform_location("size")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            color: program.uniform_location("color")?,
            rotation: program.uniform_location("rotation")?,

            program,
        })
    }
}

pub struct SpriteRenderer {
    shader: SpriteShader,
    surface: SurfaceSharedData,
}

impl SpriteRenderer {
    pub fn new() -> Result<Self, RendererError> {
        let surface = SurfaceSharedData::make_collapsed_xy_quad();

        Ok(Self {
            shader: SpriteShader::new()?,
            surface,
        })
    }

    #[must_use]
    pub fn render(&mut self,
                  state: &mut State,
                  framebuffer: &mut FrameBuffer,
                  graph: &Graph,
                  camera: &Camera,
                  white_dummy: Rc<RefCell<GpuTexture>>,
                  viewport: Rect<i32>,
                  textures: &mut TextureCache,
                  geom_map: &mut GeometryCache,
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        state.set_blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        let inv_view = camera.inv_view_matrix().unwrap();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        for node in graph.linear_iter() {
            let sprite = if let Node::Sprite(sprite) = node {
                sprite
            } else {
                continue;
            };

            let diffuse_texture = if let Some(texture) = sprite.texture() {
                if let Some(texture) = textures.get(state, texture) {
                    texture
                } else {
                    white_dummy.clone()
                }
            } else {
                white_dummy.clone()
            };

            statistics.add_draw_call(framebuffer.draw(
                state,
                viewport,
                geom_map.get(&self.surface),
                &mut self.shader.program,
                DrawParameters {
                    cull_face: CullFace::Back,
                    culling: true,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: false,
                    depth_test: true,
                    blend: true,
                },
                &[
                    (self.shader.diffuse_texture, UniformValue::Sampler {
                        index: 0,
                        texture: diffuse_texture,
                    }),
                    (self.shader.view_projection_matrix, UniformValue::Mat4(camera.view_projection_matrix())),
                    (self.shader.world_matrix, UniformValue::Mat4(node.base().global_transform())),
                    (self.shader.camera_up_vector, UniformValue::Vec3(camera_up)),
                    (self.shader.camera_side_vector, UniformValue::Vec3(camera_side)),
                    (self.shader.size, UniformValue::Float(sprite.size())),
                    (self.shader.color, UniformValue::Color(sprite.color())),
                    (self.shader.rotation, UniformValue::Float(sprite.rotation()))
                ],
            ));
        }

        statistics
    }
}