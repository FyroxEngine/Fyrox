use crate::{
    core::scope_profile,
    core::math::{
        Rect,
        mat4::Mat4,
        vec3::Vec3,
    },
    renderer::{
        framework::{
            state::State,
            geometry_buffer::{
                GeometryBuffer,
                DrawCallStatistics,
            },
            framebuffer::{
                FrameBufferTrait,
                DrawParameters,
                CullFace,
            },
            gpu_program::{
                GpuProgram,
                UniformLocation,
                UniformValue,
            },
        },
        gbuffer::GBuffer,
        surface,
        error::RendererError,
    },
    scene::{
        light::{
            Light,
            LightKind,
        },
        base::AsBase,
    },
};

struct SpotLightShader {
    program: GpuProgram,
    depth_sampler: UniformLocation,
    world_view_proj_matrix: UniformLocation,
    light_position: UniformLocation,
    light_direction: UniformLocation,
    cone_angle_cos: UniformLocation,
    light_color: UniformLocation,
    scatter_factor: UniformLocation,
    inv_proj: UniformLocation,
}

impl SpotLightShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/spot_volumetric_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");
        let program = GpuProgram::from_source("SpotVolumetricLight", vertex_source, fragment_source)?;
        Ok(Self {
            world_view_proj_matrix: program.uniform_location("worldViewProjection")?,
            depth_sampler: program.uniform_location("depthSampler")?,
            light_position: program.uniform_location("lightPosition")?,
            light_direction: program.uniform_location("lightDirection")?,
            cone_angle_cos: program.uniform_location("coneAngleCos")?,
            light_color: program.uniform_location("lightColor")?,
            scatter_factor: program.uniform_location("scatterFactor")?,
            inv_proj: program.uniform_location("invProj")?,
            program,
        })
    }
}

struct PointLightShader {
    program: GpuProgram,
    depth_sampler: UniformLocation,
    world_view_proj_matrix: UniformLocation,
    light_position: UniformLocation,
    light_radius: UniformLocation,
    light_color: UniformLocation,
    scatter_factor: UniformLocation,
    inv_proj: UniformLocation,
}

impl PointLightShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/point_volumetric_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");
        let program = GpuProgram::from_source("PointVolumetricLight", vertex_source, fragment_source)?;
        Ok(Self {
            world_view_proj_matrix: program.uniform_location("worldViewProjection")?,
            depth_sampler: program.uniform_location("depthSampler")?,
            light_position: program.uniform_location("lightPosition")?,
            inv_proj: program.uniform_location("invProj")?,
            light_radius: program.uniform_location("lightRadius")?,
            light_color: program.uniform_location("lightColor")?,
            scatter_factor: program.uniform_location("scatterFactor")?,
            program,
        })
    }
}

pub struct LightVolumeRenderer {
    spot_light_shader: SpotLightShader,
    point_light_shader: PointLightShader,
}

impl LightVolumeRenderer {
    pub fn new() -> Result<Self, RendererError> {
        Ok(Self {
            spot_light_shader: SpotLightShader::new()?,
            point_light_shader: PointLightShader::new()?,
        })
    }

    pub fn render_volume(&mut self,
                         state: &mut State,
                         light: &Light,
                         gbuffer: &mut GBuffer,
                         quad: &GeometryBuffer<surface::Vertex>,
                         view: Mat4,
                         inv_proj: Mat4,
                         viewport: Rect<i32>,
    ) -> DrawCallStatistics {
        scope_profile!();

        if !light.is_scatter_enabled() {
            return DrawCallStatistics { triangles: 0 };
        }

        let frame_matrix =
            Mat4::ortho(0.0, viewport.w as f32, viewport.h as f32, 0.0, -1.0, 1.0) *
                Mat4::scale(Vec3::new(viewport.w as f32, viewport.h as f32, 0.0));

        let position = view.transform_vector(light.base().global_position());

        match light.kind() {
            LightKind::Spot(spot) => {
                let direction = view.basis().transform_vector(-light.base().up_vector().normalized().unwrap_or(Vec3::LOOK));

                gbuffer.final_frame.draw(
                    quad,
                    state,
                    viewport,
                    &self.spot_light_shader.program,
                    DrawParameters {
                        cull_face: CullFace::Back,
                        culling: false,
                        color_write: Default::default(),
                        depth_write: false,
                        stencil_test: false,
                        depth_test: false,
                        blend: true,
                    },
                    &[
                        (self.spot_light_shader.world_view_proj_matrix, UniformValue::Mat4(frame_matrix)),
                        (self.spot_light_shader.inv_proj, UniformValue::Mat4(inv_proj)),
                        (self.spot_light_shader.cone_angle_cos, UniformValue::Float((spot.full_cone_angle() * 0.5).cos())),
                        (self.spot_light_shader.light_position, UniformValue::Vec3(position)),
                        (self.spot_light_shader.light_direction, UniformValue::Vec3(direction)),
                        (self.spot_light_shader.depth_sampler, UniformValue::Sampler { index: 0, texture: gbuffer.depth() }),
                        (self.spot_light_shader.light_color, UniformValue::Vec3(light.color().as_frgba().xyz())),
                        (self.spot_light_shader.scatter_factor, UniformValue::Vec3(light.scatter())),
                    ],
                )
            }
            LightKind::Point(point) => {
                gbuffer.final_frame.draw(
                    quad,
                    state,
                    viewport,
                    &self.point_light_shader.program,
                    DrawParameters {
                        cull_face: CullFace::Back,
                        culling: false,
                        color_write: Default::default(),
                        depth_write: false,
                        stencil_test: false,
                        depth_test: false,
                        blend: true,
                    },
                    &[
                        (self.point_light_shader.world_view_proj_matrix, UniformValue::Mat4(frame_matrix)),
                        (self.point_light_shader.inv_proj, UniformValue::Mat4(inv_proj)),
                        (self.point_light_shader.light_position, UniformValue::Vec3(position)),
                        (self.point_light_shader.depth_sampler, UniformValue::Sampler { index: 0, texture: gbuffer.depth() }),
                        (self.point_light_shader.light_radius, UniformValue::Float(point.radius())),
                        (self.point_light_shader.light_color, UniformValue::Vec3(light.color().as_frgba().xyz())),
                        (self.point_light_shader.scatter_factor, UniformValue::Vec3(light.scatter())),
                    ],
                )
            }
            _ => DrawCallStatistics { triangles: 0 }
        }
    }
}