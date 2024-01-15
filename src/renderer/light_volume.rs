use crate::{
    core::{
        algebra::{Isometry3, Matrix4, Point3, Translation, Vector3},
        math::Rect,
        pool::Handle,
        scope_profile,
        sstorage::ImmutableString,
    },
    renderer::{
        flat_shader::FlatShader,
        framework::{
            error::FrameworkError,
            framebuffer::{BlendParameters, DrawParameters, FrameBuffer},
            geometry_buffer::{ElementRange, GeometryBuffer, GeometryBufferKind},
            gpu_program::{GpuProgram, UniformLocation},
            state::{
                BlendFactor, BlendFunc, ColorMask, CompareFunc, PipelineState, StencilAction,
                StencilFunc, StencilOp,
            },
        },
        gbuffer::GBuffer,
        RenderPassStatistics,
    },
    scene::{
        graph::Graph,
        light::{point::PointLight, spot::SpotLight},
        mesh::surface::SurfaceData,
        node::Node,
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
    intensity: UniformLocation,
}

impl SpotLightShader {
    fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/spot_volumetric_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");
        let program =
            GpuProgram::from_source(state, "SpotVolumetricLight", vertex_source, fragment_source)?;
        Ok(Self {
            world_view_proj_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            depth_sampler: program
                .uniform_location(state, &ImmutableString::new("depthSampler"))?,
            light_position: program
                .uniform_location(state, &ImmutableString::new("lightPosition"))?,
            light_direction: program
                .uniform_location(state, &ImmutableString::new("lightDirection"))?,
            cone_angle_cos: program
                .uniform_location(state, &ImmutableString::new("coneAngleCos"))?,
            light_color: program.uniform_location(state, &ImmutableString::new("lightColor"))?,
            scatter_factor: program
                .uniform_location(state, &ImmutableString::new("scatterFactor"))?,
            inv_proj: program.uniform_location(state, &ImmutableString::new("invProj"))?,
            intensity: program.uniform_location(state, &ImmutableString::new("intensity"))?,
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
    intensity: UniformLocation,
}

impl PointLightShader {
    fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/point_volumetric_fs.glsl");
        let vertex_source = include_str!("shaders/flat_vs.glsl");
        let program = GpuProgram::from_source(
            state,
            "PointVolumetricLight",
            vertex_source,
            fragment_source,
        )?;
        Ok(Self {
            world_view_proj_matrix: program
                .uniform_location(state, &ImmutableString::new("worldViewProjection"))?,
            depth_sampler: program
                .uniform_location(state, &ImmutableString::new("depthSampler"))?,
            light_position: program
                .uniform_location(state, &ImmutableString::new("lightPosition"))?,
            inv_proj: program.uniform_location(state, &ImmutableString::new("invProj"))?,
            light_radius: program.uniform_location(state, &ImmutableString::new("lightRadius"))?,
            light_color: program.uniform_location(state, &ImmutableString::new("lightColor"))?,
            scatter_factor: program
                .uniform_location(state, &ImmutableString::new("scatterFactor"))?,
            intensity: program.uniform_location(state, &ImmutableString::new("intensity"))?,
            program,
        })
    }
}

pub struct LightVolumeRenderer {
    spot_light_shader: SpotLightShader,
    point_light_shader: PointLightShader,
    flat_shader: FlatShader,
    cone: GeometryBuffer,
    sphere: GeometryBuffer,
}

impl LightVolumeRenderer {
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            spot_light_shader: SpotLightShader::new(state)?,
            point_light_shader: PointLightShader::new(state)?,
            flat_shader: FlatShader::new(state)?,
            cone: GeometryBuffer::from_surface_data(
                &SurfaceData::make_cone(
                    16,
                    1.0,
                    1.0,
                    &Matrix4::new_translation(&Vector3::new(0.0, -1.0, 0.0)),
                ),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
            sphere: GeometryBuffer::from_surface_data(
                &SurfaceData::make_sphere(8, 8, 1.0, &Matrix4::identity()),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_volume(
        &mut self,
        state: &PipelineState,
        light: &Node,
        light_handle: Handle<Node>,
        gbuffer: &mut GBuffer,
        quad: &GeometryBuffer,
        view: Matrix4<f32>,
        inv_proj: Matrix4<f32>,
        view_proj: Matrix4<f32>,
        viewport: Rect<i32>,
        graph: &Graph,
        frame_buffer: &mut FrameBuffer,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        scope_profile!();

        let mut stats = RenderPassStatistics::default();

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

        if let Some(spot) = light.cast::<SpotLight>() {
            if !spot.base_light_ref().is_scatter_enabled() {
                return Ok(stats);
            }

            let direction = view.transform_vector(
                &(-light
                    .up_vector()
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_else(Vector3::z)),
            );

            // Draw cone into stencil buffer - it will mark pixels for further volumetric light
            // calculations, it will significantly reduce amount of pixels for far lights thus
            // significantly improve performance.

            let k = (spot.full_cone_angle() * 0.5 + 1.0f32.to_radians()).tan() * spot.distance();
            let light_shape_matrix = Isometry3 {
                rotation: graph.global_rotation(light_handle),
                translation: Translation {
                    vector: spot.global_position(),
                },
            }
            .to_homogeneous()
                * Matrix4::new_nonuniform_scaling(&Vector3::new(k, spot.distance(), k));
            let mvp = view_proj * light_shape_matrix;

            // Clear stencil only.
            frame_buffer.clear(state, viewport, None, None, Some(0));

            stats += frame_buffer.draw(
                &self.cone,
                state,
                viewport,
                &self.flat_shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Equal,
                        ref_value: 0xFF,
                        mask: 0xFFFF_FFFF,
                    }),
                    depth_test: true,
                    blend: None,
                    stencil_op: StencilOp {
                        fail: StencilAction::Replace,
                        zfail: StencilAction::Keep,
                        zpass: StencilAction::Replace,
                        write_mask: 0xFFFF_FFFF,
                    },
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding.set_matrix4(&self.flat_shader.wvp_matrix, &mvp);
                },
            )?;

            // Finally draw fullscreen quad, GPU will calculate scattering only on pixels that were
            // marked in stencil buffer. For distant lights it will be very low amount of pixels and
            // so distant lights won't impact performance.
            let shader = &self.spot_light_shader;
            let depth_map = gbuffer.depth();
            stats += frame_buffer.draw(
                quad,
                state,
                viewport,
                &shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Equal,
                        ref_value: 0xFF,
                        mask: 0xFFFF_FFFF,
                    }),
                    depth_test: false,
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                        ..Default::default()
                    }),
                    // Make sure to clean stencil buffer after drawing full screen quad.
                    stencil_op: StencilOp {
                        zpass: StencilAction::Zero,
                        ..Default::default()
                    },
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding
                        .set_matrix4(&shader.world_view_proj_matrix, &frame_matrix)
                        .set_matrix4(&shader.inv_proj, &inv_proj)
                        .set_f32(&shader.cone_angle_cos, (spot.full_cone_angle() * 0.5).cos())
                        .set_vector3(&shader.light_position, &position)
                        .set_vector3(&shader.light_direction, &direction)
                        .set_texture(&shader.depth_sampler, &depth_map)
                        .set_vector3(
                            &shader.light_color,
                            &spot.base_light_ref().color().srgb_to_linear_f32().xyz(),
                        )
                        .set_vector3(&shader.scatter_factor, &spot.base_light_ref().scatter())
                        .set_f32(&shader.intensity, spot.base_light_ref().intensity());
                },
            )?
        } else if let Some(point) = light.cast::<PointLight>() {
            if !point.base_light_ref().is_scatter_enabled() {
                return Ok(stats);
            }

            frame_buffer.clear(state, viewport, None, None, Some(0));

            // Radius bias is used to to slightly increase sphere radius to add small margin
            // for fadeout effect. It is set to 5%.
            let bias = 1.05;
            let k = bias * point.radius();
            let light_shape_matrix = Matrix4::new_translation(&light.global_position())
                * Matrix4::new_nonuniform_scaling(&Vector3::new(k, k, k));
            let mvp = view_proj * light_shape_matrix;

            stats += frame_buffer.draw(
                &self.sphere,
                state,
                viewport,
                &self.flat_shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Equal,
                        ref_value: 0xFF,
                        mask: 0xFFFF_FFFF,
                    }),
                    depth_test: true,
                    blend: None,
                    stencil_op: StencilOp {
                        fail: StencilAction::Replace,
                        zfail: StencilAction::Keep,
                        zpass: StencilAction::Replace,
                        write_mask: 0xFFFF_FFFF,
                    },
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding.set_matrix4(&self.flat_shader.wvp_matrix, &mvp);
                },
            )?;

            // Finally draw fullscreen quad, GPU will calculate scattering only on pixels that were
            // marked in stencil buffer. For distant lights it will be very low amount of pixels and
            // so distant lights won't impact performance.
            let shader = &self.point_light_shader;
            let depth_map = gbuffer.depth();
            stats += frame_buffer.draw(
                quad,
                state,
                viewport,
                &shader.program,
                &DrawParameters {
                    cull_face: None,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Equal,
                        ref_value: 0xFF,
                        mask: 0xFFFF_FFFF,
                    }),
                    depth_test: false,
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                        ..Default::default()
                    }),
                    // Make sure to clean stencil buffer after drawing full screen quad.
                    stencil_op: StencilOp {
                        zpass: StencilAction::Zero,
                        ..Default::default()
                    },
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding
                        .set_matrix4(&shader.world_view_proj_matrix, &frame_matrix)
                        .set_matrix4(&shader.inv_proj, &inv_proj)
                        .set_vector3(&shader.light_position, &position)
                        .set_texture(&shader.depth_sampler, &depth_map)
                        .set_f32(&shader.light_radius, point.radius())
                        .set_vector3(
                            &shader.light_color,
                            &point.base_light_ref().color().srgb_to_linear_f32().xyz(),
                        )
                        .set_vector3(&shader.scatter_factor, &point.base_light_ref().scatter())
                        .set_f32(&shader.intensity, point.base_light_ref().intensity());
                },
            )?
        }

        Ok(stats)
    }
}
