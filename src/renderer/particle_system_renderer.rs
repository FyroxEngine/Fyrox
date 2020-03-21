use crate::{
    renderer::{
        geometry_buffer::{
            GeometryBuffer,
            GeometryBufferKind,
            AttributeDefinition,
            AttributeKind,
            ElementKind,
        },
        gl,
        gpu_program::{GpuProgram, UniformLocation},
        error::RendererError,
        gpu_texture::GpuTexture,
        RenderPassStatistics,
    },
    scene::{
        node::Node,
        particle_system,
        base::AsBase,
        graph::Graph,
        camera::Camera,
    },
    core::math::{
        vec2::Vec2,
        Rect,
    },
};
use crate::renderer::TextureCache;
use crate::renderer::gpu_program::UniformValue;
use crate::renderer::framebuffer::{FrameBuffer, DrawParameters, CullFace, FrameBufferTrait};
use std::cell::RefCell;
use std::rc::Rc;
use crate::renderer::state::State;

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
        let mut program = GpuProgram::from_source("ParticleSystemShader", vertex_source, fragment_source)?;
        Ok(Self {
            view_projection_matrix: program.get_uniform_location("viewProjectionMatrix")?,
            world_matrix: program.get_uniform_location("worldMatrix")?,
            camera_side_vector: program.get_uniform_location("cameraSideVector")?,
            camera_up_vector: program.get_uniform_location("cameraUpVector")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            depth_buffer_texture: program.get_uniform_location("depthBufferTexture")?,
            inv_screen_size: program.get_uniform_location("invScreenSize")?,
            proj_params: program.get_uniform_location("projParams")?,
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

impl ParticleSystemRenderer {
    pub fn new() -> Result<Self, RendererError> {
        let mut geometry_buffer = GeometryBuffer::new(GeometryBufferKind::DynamicDraw, ElementKind::Triangle);

        geometry_buffer.bind()
            .describe_attributes(vec![
                AttributeDefinition { kind: AttributeKind::Float3, normalized: false },
                AttributeDefinition { kind: AttributeKind::Float2, normalized: false },
                AttributeDefinition { kind: AttributeKind::Float, normalized: false },
                AttributeDefinition { kind: AttributeKind::Float, normalized: false },
                AttributeDefinition { kind: AttributeKind::UnsignedByte4, normalized: true },
            ])?;

        Ok(Self {
            shader: ParticleSystemShader::new()?,
            draw_data: Default::default(),
            geometry_buffer,
            sorted_particles: Vec::new(),
        })
    }

    #[must_use]
    pub fn render(&mut self,
                  state: &mut State,
                  framebuffer: &mut FrameBuffer,
                  graph: &Graph,
                  camera: &Camera,
                  white_dummy: Rc<RefCell<GpuTexture>>,
                  depth: Rc<RefCell<GpuTexture>>,
                  frame_width: f32,
                  frame_height: f32,
                  viewport: Rect<i32>,
                  texture_cache: &mut TextureCache,
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        unsafe {
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let inv_view = camera.inv_view_matrix().unwrap();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        for node in graph.linear_iter() {
            let particle_system = if let Node::ParticleSystem(particle_system) = node {
                particle_system
            } else {
                continue;
            };

            particle_system.generate_draw_data(&mut self.sorted_particles,
                                               &mut self.draw_data,
                                               &camera.base().global_position());

            self.geometry_buffer
                .bind()
                .set_triangles(self.draw_data.get_triangles())
                .set_vertices(self.draw_data.get_vertices());

            statistics.add_draw_call(
                framebuffer.draw(
                    state,
                    viewport,
                    &mut self.geometry_buffer,
                    &mut self.shader.program,
                    DrawParameters {
                        cull_face: CullFace::Front,
                        culling: false,
                        color_write: (true, true, true, true),
                        depth_write: false,
                        stencil_test: false,
                        depth_test: true,
                        blend: true,
                    },
                    &[
                        (self.shader.depth_buffer_texture, UniformValue::Sampler { index: 0, texture: depth.clone() }),
                        (self.shader.diffuse_texture, UniformValue::Sampler {
                            index: 1,
                            texture: if let Some(texture) = particle_system.texture() {
                                if let Some(texture) = texture_cache.get(texture) {
                                    texture
                                } else {
                                    white_dummy.clone()
                                }
                            } else {
                                white_dummy.clone()
                            },
                        }),
                        (self.shader.camera_side_vector, UniformValue::Vec3(camera_side)),
                        (self.shader.camera_up_vector, UniformValue::Vec3(camera_up)),
                        (self.shader.view_projection_matrix, UniformValue::Mat4(camera.view_projection_matrix())),
                        (self.shader.world_matrix, UniformValue::Mat4(node.base().global_transform())),
                        (self.shader.inv_screen_size, UniformValue::Vec2(Vec2::new(1.0 / frame_width, 1.0 / frame_height))),
                        (self.shader.proj_params, UniformValue::Vec2(Vec2::new(camera.z_far(), camera.z_near())))],
                ));
        }

        unsafe {
            gl::Disable(gl::BLEND);
            gl::DepthMask(gl::TRUE);
        }

        statistics
    }
}
