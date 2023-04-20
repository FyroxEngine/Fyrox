use crate::core::sstorage::ImmutableString;
use crate::renderer::framework::framebuffer::BlendParameters;
use crate::renderer::framework::geometry_buffer::ElementRange;
use crate::renderer::framework::state::{BlendFactor, BlendFunc};
use crate::scene::particle_system::ParticleSystem;
use crate::{
    core::{algebra::Vector2, math::Matrix4Ext, math::Rect, scope_profile},
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{DrawParameters, FrameBuffer},
        geometry_buffer::{
            AttributeDefinition, AttributeKind, BufferBuilder, ElementKind, GeometryBuffer,
            GeometryBufferBuilder, GeometryBufferKind,
        },
        gpu_program::{GpuProgram, UniformLocation},
        gpu_texture::GpuTexture,
        state::PipelineState,
    },
    renderer::{RenderPassStatistics, TextureCache},
    scene::{camera::Camera, graph::Graph, particle_system},
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
    soft_boundary_sharpness_factor: UniformLocation,
}

impl ParticleSystemShader {
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let vertex_source = include_str!("shaders/particle_system_vs.glsl");
        let fragment_source = include_str!("shaders/particle_system_fs.glsl");
        let program = GpuProgram::from_source(
            state,
            "ParticleSystemShader",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            view_projection_matrix: program
                .uniform_location(state, &ImmutableString::new("viewProjectionMatrix"))?,
            world_matrix: program.uniform_location(state, &ImmutableString::new("worldMatrix"))?,
            camera_side_vector: program
                .uniform_location(state, &ImmutableString::new("cameraSideVector"))?,
            camera_up_vector: program
                .uniform_location(state, &ImmutableString::new("cameraUpVector"))?,
            diffuse_texture: program
                .uniform_location(state, &ImmutableString::new("diffuseTexture"))?,
            depth_buffer_texture: program
                .uniform_location(state, &ImmutableString::new("depthBufferTexture"))?,
            inv_screen_size: program
                .uniform_location(state, &ImmutableString::new("invScreenSize"))?,
            proj_params: program.uniform_location(state, &ImmutableString::new("projParams"))?,
            soft_boundary_sharpness_factor: program
                .uniform_location(state, &ImmutableString::new("softBoundarySharpnessFactor"))?,
            program,
        })
    }
}

pub struct ParticleSystemRenderer {
    shader: ParticleSystemShader,
    draw_data: particle_system::draw::DrawData,
    geometry_buffer: GeometryBuffer,
    sorted_particles: Vec<u32>,
}

pub(crate) struct ParticleSystemRenderContext<'a, 'b, 'c> {
    pub state: &'a mut PipelineState,
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
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let geometry_buffer = GeometryBufferBuilder::new(ElementKind::Triangle)
            .with_buffer_builder(
                BufferBuilder::new::<crate::scene::particle_system::draw::Vertex>(
                    GeometryBufferKind::DynamicDraw,
                    None,
                )
                .with_attribute(AttributeDefinition {
                    location: 0,
                    kind: AttributeKind::Float3,
                    normalized: false,
                    divisor: 0,
                })
                .with_attribute(AttributeDefinition {
                    location: 1,
                    kind: AttributeKind::Float2,
                    normalized: false,
                    divisor: 0,
                })
                .with_attribute(AttributeDefinition {
                    location: 2,
                    kind: AttributeKind::Float,
                    normalized: false,
                    divisor: 0,
                })
                .with_attribute(AttributeDefinition {
                    location: 3,
                    kind: AttributeKind::Float,
                    normalized: false,
                    divisor: 0,
                })
                .with_attribute(AttributeDefinition {
                    location: 4,
                    kind: AttributeKind::UnsignedByte4,
                    normalized: true,
                    divisor: 0,
                }),
            )
            .build(state)?;

        Ok(Self {
            shader: ParticleSystemShader::new(state)?,
            draw_data: Default::default(),
            geometry_buffer,
            sorted_particles: Vec::new(),
        })
    }

    pub(crate) fn render(
        &mut self,
        args: ParticleSystemRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
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

        let inv_view = camera.inv_view_matrix().unwrap();
        let view_proj = camera.view_projection_matrix();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        let inv_screen_size = Vector2::new(1.0 / frame_width, 1.0 / frame_height);
        let proj_params = Vector2::new(camera.projection().z_far(), camera.projection().z_near());

        for particle_system in graph
            .linear_iter()
            .filter_map(|n| n.cast::<ParticleSystem>())
        {
            particle_system.generate_draw_data(
                &mut self.sorted_particles,
                &mut self.draw_data,
                &camera.global_position(),
            );

            self.geometry_buffer
                .set_buffer_data(state, 0, self.draw_data.vertices());
            self.geometry_buffer
                .bind(state)
                .set_triangles(self.draw_data.triangles());

            let global_transform = particle_system.global_transform();

            let draw_params = DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: true,
                blend: Some(BlendParameters {
                    func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                    ..Default::default()
                }),
                stencil_op: Default::default(),
            };

            let diffuse_texture = particle_system
                .texture_ref()
                .and_then(|t| texture_cache.get(state, t))
                .unwrap_or_else(|| white_dummy.clone());

            statistics += framebuffer.draw(
                &self.geometry_buffer,
                state,
                viewport,
                &self.shader.program,
                &draw_params,
                ElementRange::Full,
                |mut program_binding| {
                    program_binding
                        .set_texture(&self.shader.depth_buffer_texture, &depth)
                        .set_texture(&self.shader.diffuse_texture, &diffuse_texture)
                        .set_vector3(&self.shader.camera_side_vector, &camera_side)
                        .set_vector3(&self.shader.camera_up_vector, &camera_up)
                        .set_matrix4(&self.shader.view_projection_matrix, &view_proj)
                        .set_matrix4(&self.shader.world_matrix, &global_transform)
                        .set_vector2(&self.shader.inv_screen_size, &inv_screen_size)
                        .set_vector2(&self.shader.proj_params, &proj_params)
                        .set_f32(
                            &self.shader.soft_boundary_sharpness_factor,
                            particle_system.soft_boundary_sharpness_factor(),
                        );
                },
            )?;
        }

        Ok(statistics)
    }
}
