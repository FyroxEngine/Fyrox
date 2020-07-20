use crate::{
    core::{
        math::{vec2::Vec2, Rect},
        scope_profile,
    },
    renderer::{
        error::RendererError,
        framework::{
            framebuffer::{CullFace, DrawParameters, FrameBuffer, FrameBufferTrait},
            geometry_buffer::{
                AttributeDefinition, AttributeKind, ElementKind, GeometryBuffer, GeometryBufferKind,
            },
            gl,
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::GpuTexture,
            state::State,
        },
        RenderPassStatistics, TextureCache,
    },
    scene::{camera::Camera, graph::Graph, node::Node, particle_system},
};
use std::{cell::RefCell, rc::Rc};

struct ParticleSystemShader {
    program: GpuProgram,
    view_projection_matrix: UniformLocation,
    world_matrix: UniformLocation,
    camera_side_vector: UniformLocation,
    camera_up_vector: UniformLocation,
    diffuse_texture: UniformLocation,
    depth_buffer_texture: UniformLocation,
    inv_screen_size: UniformLocation,
    proj_params: UniformLocation,
}

impl ParticleSystemShader {
    fn new() -> Result<Self, RendererError> {
        let vertex_source = include_str!("shaders/particle_system_vs.glsl");
        let fragment_source = include_str!("shaders/particle_system_fs.glsl");
        let program =
            GpuProgram::from_source("ParticleSystemShader", vertex_source, fragment_source)?;
        Ok(Self {
            view_projection_matrix: program.uniform_location("viewProjectionMatrix")?,
            world_matrix: program.uniform_location("worldMatrix")?,
            camera_side_vector: program.uniform_location("cameraSideVector")?,
            camera_up_vector: program.uniform_location("cameraUpVector")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            depth_buffer_texture: program.uniform_location("depthBufferTexture")?,
            inv_screen_size: program.uniform_location("invScreenSize")?,
            proj_params: program.uniform_location("projParams")?,
            program,
        })
    }
}

pub struct ParticleSystemRenderer {
    shader: ParticleSystemShader,
    draw_data: particle_system::DrawData,
    geometry_buffer: GeometryBuffer<particle_system::Vertex>,
    sorted_particles: Vec<u32>,
}

pub struct ParticleSystemRenderContext<'a, 'b, 'c> {
    pub state: &'a mut State,
    pub framebuffer: &'b mut FrameBuffer,
    pub graph: &'c Graph,
    pub camera: &'c Camera,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub depth: Rc<RefCell<GpuTexture>>,
    pub frame_width: f32,
    pub frame_height: f32,
    pub viewport: Rect<i32>,
    pub texture_cache: &'a mut TextureCache,
}

impl ParticleSystemRenderer {
    pub fn new(state: &mut State) -> Result<Self, RendererError> {
        let geometry_buffer =
            GeometryBuffer::new(GeometryBufferKind::DynamicDraw, ElementKind::Triangle);

        geometry_buffer.bind(state).describe_attributes(vec![
            AttributeDefinition {
                kind: AttributeKind::Float3,
                normalized: false,
            },
            AttributeDefinition {
                kind: AttributeKind::Float2,
                normalized: false,
            },
            AttributeDefinition {
                kind: AttributeKind::Float,
                normalized: false,
            },
            AttributeDefinition {
                kind: AttributeKind::Float,
                normalized: false,
            },
            AttributeDefinition {
                kind: AttributeKind::UnsignedByte4,
                normalized: true,
            },
        ])?;

        Ok(Self {
            shader: ParticleSystemShader::new()?,
            draw_data: Default::default(),
            geometry_buffer,
            sorted_particles: Vec::new(),
        })
    }

    #[must_use]
    pub fn render(&mut self, args: ParticleSystemRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let ParticleSystemRenderContext {
            state,
            framebuffer,
            graph,
            camera,
            white_dummy,
            depth,
            frame_width,
            frame_height,
            viewport,
            texture_cache,
        } = args;

        state.set_blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        let inv_view = camera.inv_view_matrix().unwrap();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        for node in graph.linear_iter() {
            let particle_system = if let Node::ParticleSystem(particle_system) = node {
                particle_system
            } else {
                continue;
            };

            particle_system.generate_draw_data(
                &mut self.sorted_particles,
                &mut self.draw_data,
                &camera.global_position(),
            );

            self.geometry_buffer
                .bind(state)
                .set_triangles(self.draw_data.triangles())
                .set_vertices(self.draw_data.vertices());

            let uniforms = [
                (
                    self.shader.depth_buffer_texture,
                    UniformValue::Sampler {
                        index: 0,
                        texture: depth.clone(),
                    },
                ),
                (
                    self.shader.diffuse_texture,
                    UniformValue::Sampler {
                        index: 1,
                        texture: if let Some(texture) = particle_system.texture() {
                            if let Some(texture) = texture_cache.get(state, texture) {
                                texture
                            } else {
                                white_dummy.clone()
                            }
                        } else {
                            white_dummy.clone()
                        },
                    },
                ),
                (
                    self.shader.camera_side_vector,
                    UniformValue::Vec3(camera_side),
                ),
                (self.shader.camera_up_vector, UniformValue::Vec3(camera_up)),
                (
                    self.shader.view_projection_matrix,
                    UniformValue::Mat4(camera.view_projection_matrix()),
                ),
                (
                    self.shader.world_matrix,
                    UniformValue::Mat4(node.global_transform()),
                ),
                (
                    self.shader.inv_screen_size,
                    UniformValue::Vec2(Vec2::new(1.0 / frame_width, 1.0 / frame_height)),
                ),
                (
                    self.shader.proj_params,
                    UniformValue::Vec2(Vec2::new(camera.z_far(), camera.z_near())),
                ),
            ];

            let draw_params = DrawParameters {
                cull_face: CullFace::Front,
                culling: false,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: false,
                depth_test: true,
                blend: true,
            };

            statistics += framebuffer.draw(
                &self.geometry_buffer,
                state,
                viewport,
                &self.shader.program,
                draw_params,
                &uniforms,
            );
        }

        statistics
    }
}
