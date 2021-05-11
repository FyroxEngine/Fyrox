use crate::core::algebra::{Isometry3, Translation};
use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector3},
        math::Rect,
        pool::Handle,
        scope_profile,
    },
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{CullFace, DrawParameters, FrameBufferTrait},
        gpu_program::{GpuProgram, UniformLocation, UniformValue},
        state::{ColorMask, PipelineState, StencilFunc, StencilOp},
    },
    renderer::{
        flat_shader::FlatShader, gbuffer::GBuffer, surface::SurfaceSharedData, GeometryCache,
        RenderPassStatistics,
    },
    scene::{graph::Graph, light::Light, node::Node},
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
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/spot_volumetric_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");
        let program =
            GpuProgram::from_source(state, "SpotVolumetricLight", vertex_source, fragment_source)?;
        Ok(Self {
            world_view_proj_matrix: program.uniform_location(state, "worldViewProjection")?,
            depth_sampler: program.uniform_location(state, "depthSampler")?,
            light_position: program.uniform_location(state, "lightPosition")?,
            light_direction: program.uniform_location(state, "lightDirection")?,
            cone_angle_cos: program.uniform_location(state, "coneAngleCos")?,
            light_color: program.uniform_location(state, "lightColor")?,
            scatter_factor: program.uniform_location(state, "scatterFactor")?,
            inv_proj: program.uniform_location(state, "invProj")?,
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
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/point_volumetric_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");
        let program = GpuProgram::from_source(
            state,
            "PointVolumetricLight",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            world_view_proj_matrix: program.uniform_location(state, "worldViewProjection")?,
            depth_sampler: program.uniform_location(state, "depthSampler")?,
            light_position: program.uniform_location(state, "lightPosition")?,
            inv_proj: program.uniform_location(state, "invProj")?,
            light_radius: program.uniform_location(state, "lightRadius")?,
            light_color: program.uniform_location(state, "lightColor")?,
            scatter_factor: program.uniform_location(state, "scatterFactor")?,
            program,
        })
    }
}

pub struct LightVolumeRenderer {
    spot_light_shader: SpotLightShader,
    point_light_shader: PointLightShader,
    flat_shader: FlatShader,
    cone: SurfaceSharedData,
    sphere: SurfaceSharedData,
}

impl LightVolumeRenderer {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            spot_light_shader: SpotLightShader::new(state)?,
            point_light_shader: PointLightShader::new(state)?,
            flat_shader: FlatShader::new(state)?,
            cone: SurfaceSharedData::make_cone(
                16,
                1.0,
                1.0,
                &Matrix4::new_translation(&Vector3::new(0.0, -1.0, 0.0)),
            ),
            sphere: SurfaceSharedData::make_sphere(8, 8, 1.0, &Matrix4::identity()),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate) fn render_volume(
        &mut self,
        state: &mut PipelineState,
        light: &Light,
        light_handle: Handle<Node>,
        gbuffer: &mut GBuffer,
        quad: &SurfaceSharedData,
        geom_cache: &mut GeometryCache,
        view: Matrix4<f32>,
        inv_proj: Matrix4<f32>,
        view_proj: Matrix4<f32>,
        viewport: Rect<i32>,
        graph: &Graph,
    ) -> RenderPassStatistics {
        scope_profile!();

        let mut stats = RenderPassStatistics::default();

        if !light.is_scatter_enabled() {
            return stats;
        }

        let frame_matrix = Matrix4::new_orthographic(
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
        ));

        let position = view
            .transform_point(&Point3::from(light.global_position()))
            .coords;

        match light {
            Light::Spot(spot) => {
                let direction = view.transform_vector(
                    &(-light
                        .up_vector()
                        .try_normalize(f32::EPSILON)
                        .unwrap_or_else(Vector3::z)),
                );

                // Draw cone into stencil buffer - it will mark pixels for further volumetric light
                // calculations, it will significantly reduce amount of pixels for far lights thus
                // significantly improve performance.

                let k =
                    (spot.full_cone_angle() * 0.5 + 1.0f32.to_radians()).tan() * spot.distance();
                let light_shape_matrix = Isometry3 {
                    rotation: graph.global_rotation(light_handle),
                    translation: Translation {
                        vector: spot.global_position(),
                    },
                }
                .to_homogeneous()
                    * Matrix4::new_nonuniform_scaling(&Vector3::new(k, spot.distance(), k));
                let mvp = view_proj * light_shape_matrix;

                gbuffer
                    .final_frame
                    .clear(state, viewport, None, None, Some(0));

                state.set_stencil_mask(0xFFFF_FFFF);
                state.set_stencil_func(StencilFunc {
                    func: glow::EQUAL,
                    ref_value: 0xFF,
                    mask: 0xFFFF_FFFF,
                });
                state.set_stencil_op(StencilOp {
                    fail: glow::REPLACE,
                    zfail: glow::KEEP,
                    zpass: glow::REPLACE,
                });

                stats += gbuffer.final_frame.draw(
                    geom_cache.get(state, &self.cone),
                    state,
                    viewport,
                    &self.flat_shader.program,
                    &DrawParameters {
                        cull_face: CullFace::Back,
                        culling: false,
                        color_write: ColorMask::all(false),
                        depth_write: false,
                        stencil_test: true,
                        depth_test: true,
                        blend: false,
                    },
                    &[(
                        self.flat_shader.wvp_matrix.clone(),
                        UniformValue::Matrix4(&mvp),
                    )],
                );

                // Make sure to clean stencil buffer after drawing full screen quad.
                state.set_stencil_op(StencilOp {
                    zpass: glow::ZERO,
                    ..Default::default()
                });

                // Finally draw fullscreen quad, GPU will calculate scattering only on pixels that were
                // marked in stencil buffer. For distant lights it will be very low amount of pixels and
                // so distant lights won't impact performance.
                stats += gbuffer.final_frame.draw(
                    geom_cache.get(state, quad),
                    state,
                    viewport,
                    &self.spot_light_shader.program,
                    &DrawParameters {
                        cull_face: CullFace::Back,
                        culling: false,
                        color_write: Default::default(),
                        depth_write: false,
                        stencil_test: true,
                        depth_test: false,
                        blend: true,
                    },
                    &[
                        (
                            self.spot_light_shader.world_view_proj_matrix.clone(),
                            UniformValue::Matrix4(&frame_matrix),
                        ),
                        (
                            self.spot_light_shader.inv_proj.clone(),
                            UniformValue::Matrix4(&inv_proj),
                        ),
                        (
                            self.spot_light_shader.cone_angle_cos.clone(),
                            UniformValue::Float((spot.full_cone_angle() * 0.5).cos()),
                        ),
                        (
                            self.spot_light_shader.light_position.clone(),
                            UniformValue::Vector3(&position),
                        ),
                        (
                            self.spot_light_shader.light_direction.clone(),
                            UniformValue::Vector3(&direction),
                        ),
                        (
                            self.spot_light_shader.depth_sampler.clone(),
                            UniformValue::Sampler {
                                index: 0,
                                texture: gbuffer.depth(),
                            },
                        ),
                        (
                            self.spot_light_shader.light_color.clone(),
                            UniformValue::Vector3(&light.color().as_frgba().xyz()),
                        ),
                        (
                            self.spot_light_shader.scatter_factor.clone(),
                            UniformValue::Vector3(&light.scatter()),
                        ),
                    ],
                )
            }
            Light::Point(point) => {
                gbuffer
                    .final_frame
                    .clear(state, viewport, None, None, Some(0));

                state.set_stencil_mask(0xFFFF_FFFF);
                state.set_stencil_func(StencilFunc {
                    func: glow::EQUAL,
                    ref_value: 0xFF,
                    mask: 0xFFFF_FFFF,
                });
                state.set_stencil_op(StencilOp {
                    fail: glow::REPLACE,
                    zfail: glow::KEEP,
                    zpass: glow::REPLACE,
                });

                // Radius bias is used to to slightly increase sphere radius to add small margin
                // for fadeout effect. It is set to 5%.
                let bias = 1.05;
                let k = bias * point.radius();
                let light_shape_matrix = Matrix4::new_translation(&light.global_position())
                    * Matrix4::new_nonuniform_scaling(&Vector3::new(k, k, k));
                let mvp = view_proj * light_shape_matrix;

                stats += gbuffer.final_frame.draw(
                    geom_cache.get(state, &self.sphere),
                    state,
                    viewport,
                    &self.flat_shader.program,
                    &DrawParameters {
                        cull_face: CullFace::Back,
                        culling: false,
                        color_write: ColorMask::all(false),
                        depth_write: false,
                        stencil_test: true,
                        depth_test: true,
                        blend: false,
                    },
                    &[(
                        self.flat_shader.wvp_matrix.clone(),
                        UniformValue::Matrix4(&mvp),
                    )],
                );

                // Make sure to clean stencil buffer after drawing full screen quad.
                state.set_stencil_op(StencilOp {
                    zpass: glow::ZERO,
                    ..Default::default()
                });

                // Finally draw fullscreen quad, GPU will calculate scattering only on pixels that were
                // marked in stencil buffer. For distant lights it will be very low amount of pixels and
                // so distant lights won't impact performance.
                stats += gbuffer.final_frame.draw(
                    geom_cache.get(state, quad),
                    state,
                    viewport,
                    &self.point_light_shader.program,
                    &DrawParameters {
                        cull_face: CullFace::Back,
                        culling: false,
                        color_write: Default::default(),
                        depth_write: false,
                        stencil_test: true,
                        depth_test: false,
                        blend: true,
                    },
                    &[
                        (
                            self.point_light_shader.world_view_proj_matrix.clone(),
                            UniformValue::Matrix4(&frame_matrix),
                        ),
                        (
                            self.point_light_shader.inv_proj.clone(),
                            UniformValue::Matrix4(&inv_proj),
                        ),
                        (
                            self.point_light_shader.light_position.clone(),
                            UniformValue::Vector3(&position),
                        ),
                        (
                            self.point_light_shader.depth_sampler.clone(),
                            UniformValue::Sampler {
                                index: 0,
                                texture: gbuffer.depth(),
                            },
                        ),
                        (
                            self.point_light_shader.light_radius.clone(),
                            UniformValue::Float(point.radius()),
                        ),
                        (
                            self.point_light_shader.light_color.clone(),
                            UniformValue::Vector3(&light.color().as_frgba().xyz()),
                        ),
                        (
                            self.point_light_shader.scatter_factor.clone(),
                            UniformValue::Vector3(&light.scatter()),
                        ),
                    ],
                )
            }
            _ => (),
        }

        stats
    }
}
