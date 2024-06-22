use crate::renderer::LightingStatistics;
use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        color::Color,
        math::{frustum::Frustum, Matrix4Ext, Rect, TriangleDefinition},
        scope_profile,
    },
    graph::SceneGraph,
    renderer::{
        cache::shader::ShaderCache,
        flat_shader::FlatShader,
        framework::{
            error::FrameworkError,
            framebuffer::{BlendParameters, CullFace, DrawParameters, FrameBuffer},
            geometry_buffer::{ElementRange, GeometryBuffer, GeometryBufferKind},
            gpu_texture::GpuTexture,
            state::{
                BlendFactor, BlendFunc, ColorMask, CompareFunc, PipelineState, StencilAction,
                StencilFunc, StencilOp,
            },
        },
        gbuffer::GBuffer,
        light::{
            ambient::AmbientLightShader, directional::DirectionalLightShader,
            point::PointLightShader, spot::SpotLightShader,
        },
        light_volume::LightVolumeRenderer,
        shadow::{
            csm::{CsmRenderContext, CsmRenderer},
            point::{PointShadowMapRenderContext, PointShadowMapRenderer},
            spot::SpotShadowMapRenderer,
        },
        skybox_shader::SkyboxShader,
        ssao::ScreenSpaceAmbientOcclusionRenderer,
        storage::MatrixStorageCache,
        GeometryCache, QualitySettings, RenderPassStatistics, TextureCache,
    },
    scene::{
        camera::Camera,
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::{
            buffer::{TriangleBuffer, VertexBuffer},
            surface::SurfaceData,
            vertex::SimpleVertex,
        },
        Scene,
    },
};
use std::{cell::RefCell, rc::Rc};

pub mod ambient;
pub mod directional;
pub mod point;
pub mod spot;

pub struct DeferredLightRenderer {
    pub ssao_renderer: ScreenSpaceAmbientOcclusionRenderer,
    spot_light_shader: SpotLightShader,
    point_light_shader: PointLightShader,
    directional_light_shader: DirectionalLightShader,
    ambient_light_shader: AmbientLightShader,
    quad: GeometryBuffer,
    sphere: GeometryBuffer,
    skybox: GeometryBuffer,
    flat_shader: FlatShader,
    skybox_shader: SkyboxShader,
    spot_shadow_map_renderer: SpotShadowMapRenderer,
    point_shadow_map_renderer: PointShadowMapRenderer,
    csm_renderer: CsmRenderer,
    light_volume: LightVolumeRenderer,
}

pub(crate) struct DeferredRendererContext<'a> {
    pub state: &'a PipelineState,
    pub scene: &'a Scene,
    pub camera: &'a Camera,
    pub gbuffer: &'a mut GBuffer,
    pub ambient_color: Color,
    pub settings: &'a QualitySettings,
    pub textures: &'a mut TextureCache,
    pub geometry_cache: &'a mut GeometryCache,
    pub frame_buffer: &'a mut FrameBuffer,
    pub shader_cache: &'a mut ShaderCache,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub black_dummy: Rc<RefCell<GpuTexture>>,
    pub volume_dummy: Rc<RefCell<GpuTexture>>,
    pub matrix_storage: &'a mut MatrixStorageCache,
}

impl DeferredLightRenderer {
    pub fn new(
        state: &PipelineState,
        frame_size: (u32, u32),
        settings: &QualitySettings,
    ) -> Result<Self, FrameworkError> {
        let vertices = vec![
            // Front
            SimpleVertex::new(-0.5, 0.5, -0.5),
            SimpleVertex::new(0.5, 0.5, -0.5),
            SimpleVertex::new(0.5, -0.5, -0.5),
            SimpleVertex::new(-0.5, -0.5, -0.5),
            // Back
            SimpleVertex::new(0.5, 0.5, 0.5),
            SimpleVertex::new(-0.5, 0.5, 0.5),
            SimpleVertex::new(-0.5, -0.5, 0.5),
            SimpleVertex::new(0.5, -0.5, 0.5),
            // Left
            SimpleVertex::new(0.5, 0.5, -0.5),
            SimpleVertex::new(0.5, 0.5, 0.5),
            SimpleVertex::new(0.5, -0.5, 0.5),
            SimpleVertex::new(0.5, -0.5, -0.5),
            // Right
            SimpleVertex::new(-0.5, 0.5, 0.5),
            SimpleVertex::new(-0.5, 0.5, -0.5),
            SimpleVertex::new(-0.5, -0.5, -0.5),
            SimpleVertex::new(-0.5, -0.5, 0.5),
            // Up
            SimpleVertex::new(-0.5, 0.5, 0.5),
            SimpleVertex::new(0.5, 0.5, 0.5),
            SimpleVertex::new(0.5, 0.5, -0.5),
            SimpleVertex::new(-0.5, 0.5, -0.5),
            // Down
            SimpleVertex::new(-0.5, -0.5, 0.5),
            SimpleVertex::new(0.5, -0.5, 0.5),
            SimpleVertex::new(0.5, -0.5, -0.5),
            SimpleVertex::new(-0.5, -0.5, -0.5),
        ];

        let quality_defaults = QualitySettings::default();

        Ok(Self {
            ssao_renderer: ScreenSpaceAmbientOcclusionRenderer::new(
                state,
                frame_size.0 as usize,
                frame_size.1 as usize,
            )?,
            spot_light_shader: SpotLightShader::new(state)?,
            point_light_shader: PointLightShader::new(state)?,
            directional_light_shader: DirectionalLightShader::new(state)?,
            ambient_light_shader: AmbientLightShader::new(state)?,
            quad: GeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
            skybox: GeometryBuffer::from_surface_data(
                &SurfaceData::new(
                    VertexBuffer::new(vertices.len(), vertices).unwrap(),
                    TriangleBuffer::new(vec![
                        TriangleDefinition([0, 1, 2]),
                        TriangleDefinition([0, 2, 3]),
                        TriangleDefinition([4, 5, 6]),
                        TriangleDefinition([4, 6, 7]),
                        TriangleDefinition([8, 9, 10]),
                        TriangleDefinition([8, 10, 11]),
                        TriangleDefinition([12, 13, 14]),
                        TriangleDefinition([12, 14, 15]),
                        TriangleDefinition([16, 17, 18]),
                        TriangleDefinition([16, 18, 19]),
                        TriangleDefinition([20, 21, 22]),
                        TriangleDefinition([20, 22, 23]),
                    ]),
                ),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
            sphere: GeometryBuffer::from_surface_data(
                &SurfaceData::make_sphere(6, 6, 1.0, &Matrix4::identity()),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
            flat_shader: FlatShader::new(state)?,
            skybox_shader: SkyboxShader::new(state)?,
            spot_shadow_map_renderer: SpotShadowMapRenderer::new(
                state,
                settings.spot_shadow_map_size,
                quality_defaults.spot_shadow_map_precision,
            )?,
            point_shadow_map_renderer: PointShadowMapRenderer::new(
                state,
                settings.point_shadow_map_size,
                quality_defaults.point_shadow_map_precision,
            )?,
            light_volume: LightVolumeRenderer::new(state)?,
            csm_renderer: CsmRenderer::new(
                state,
                quality_defaults.csm_settings.size,
                quality_defaults.csm_settings.precision,
            )?,
        })
    }

    pub fn set_quality_settings(
        &mut self,
        state: &PipelineState,
        settings: &QualitySettings,
    ) -> Result<(), FrameworkError> {
        if settings.spot_shadow_map_size != self.spot_shadow_map_renderer.base_size()
            || settings.spot_shadow_map_precision != self.spot_shadow_map_renderer.precision()
        {
            self.spot_shadow_map_renderer = SpotShadowMapRenderer::new(
                state,
                settings.spot_shadow_map_size,
                settings.spot_shadow_map_precision,
            )?;
        }
        if settings.point_shadow_map_size != self.point_shadow_map_renderer.base_size()
            || settings.point_shadow_map_precision != self.point_shadow_map_renderer.precision()
        {
            self.point_shadow_map_renderer = PointShadowMapRenderer::new(
                state,
                settings.point_shadow_map_size,
                settings.point_shadow_map_precision,
            )?;
        }
        if settings.csm_settings.precision != self.csm_renderer.precision()
            || settings.csm_settings.size != self.csm_renderer.size()
        {
            self.csm_renderer = CsmRenderer::new(
                state,
                settings.csm_settings.size,
                settings.csm_settings.precision,
            )?;
        }
        self.ssao_renderer.set_radius(settings.ssao_radius);
        Ok(())
    }

    pub fn set_frame_size(
        &mut self,
        state: &PipelineState,
        frame_size: (u32, u32),
    ) -> Result<(), FrameworkError> {
        self.ssao_renderer = ScreenSpaceAmbientOcclusionRenderer::new(
            state,
            frame_size.0 as usize,
            frame_size.1 as usize,
        )?;
        Ok(())
    }

    pub(crate) fn render(
        &mut self,
        args: DeferredRendererContext,
    ) -> Result<(RenderPassStatistics, LightingStatistics), FrameworkError> {
        scope_profile!();

        let mut pass_stats = RenderPassStatistics::default();
        let mut light_stats = LightingStatistics::default();

        let DeferredRendererContext {
            state,
            scene,
            camera,
            gbuffer,
            shader_cache,
            normal_dummy,
            white_dummy,
            ambient_color,
            settings,
            textures,
            geometry_cache,
            frame_buffer,
            black_dummy,
            volume_dummy,
            matrix_storage,
        } = args;

        let viewport = Rect::new(0, 0, gbuffer.width, gbuffer.height);
        let frustum = Frustum::from_view_projection_matrix(camera.view_projection_matrix())
            .unwrap_or_default();

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

        let projection_matrix = camera.projection_matrix();
        let view_projection = camera.view_projection_matrix();
        let inv_projection = projection_matrix.try_inverse().unwrap_or_default();
        let inv_view_projection = view_projection.try_inverse().unwrap_or_default();
        let camera_global_position = camera.global_position();

        // Fill SSAO map.
        if settings.use_ssao {
            pass_stats += self.ssao_renderer.render(
                state,
                gbuffer,
                projection_matrix,
                camera.view_matrix().basis(),
            )?;
        }

        // Render skybox (if any).
        if let Some(skybox) = camera.skybox_ref() {
            let size = camera.projection().z_far() / 2.0f32.sqrt();
            let scale = Matrix4::new_scaling(size);
            let wvp = Matrix4::new_translation(&camera.global_position()) * scale;

            if let Some(gpu_texture) = skybox
                .cubemap_ref()
                .and_then(|cube_map| textures.get(state, cube_map))
            {
                let shader = &self.skybox_shader;
                pass_stats += frame_buffer.draw(
                    &self.skybox,
                    state,
                    viewport,
                    &shader.program,
                    &DrawParameters {
                        cull_face: None,
                        color_write: Default::default(),
                        depth_write: false,
                        stencil_test: None,
                        depth_test: false,
                        blend: None,
                        stencil_op: Default::default(),
                    },
                    ElementRange::Specific {
                        offset: 0,
                        count: 12,
                    },
                    |mut program_binding| {
                        program_binding
                            .set_texture(&shader.cubemap_texture, gpu_texture)
                            .set_matrix4(&shader.wvp_matrix, &(view_projection * wvp));
                    },
                )?;
            }
        }

        // Ambient light.
        let gbuffer_depth_map = gbuffer.depth();
        let gbuffer_diffuse_map = gbuffer.diffuse_texture();
        let gbuffer_normal_map = gbuffer.normal_texture();
        let gbuffer_material_map = gbuffer.material_texture();
        let gbuffer_ambient_map = gbuffer.ambient_texture();
        let ao_map = self.ssao_renderer.ao_map();

        pass_stats += frame_buffer.draw(
            &self.quad,
            state,
            viewport,
            &self.ambient_light_shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: false,
                blend: Some(BlendParameters {
                    func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                    ..Default::default()
                }),
                stencil_op: Default::default(),
            },
            ElementRange::Full,
            |mut program_binding| {
                program_binding
                    .set_matrix4(&self.ambient_light_shader.wvp_matrix, &frame_matrix)
                    .set_linear_color(&self.ambient_light_shader.ambient_color, &ambient_color)
                    .set_texture(
                        &self.ambient_light_shader.diffuse_texture,
                        &gbuffer_diffuse_map,
                    )
                    .set_texture(
                        &self.ambient_light_shader.ao_sampler,
                        if settings.use_ssao {
                            &ao_map
                        } else {
                            &white_dummy
                        },
                    )
                    .set_texture(
                        &self.ambient_light_shader.ambient_texture,
                        &gbuffer_ambient_map,
                    );
            },
        )?;

        for (light_handle, light) in scene.graph.pair_iter() {
            if !light.global_visibility() || !light.is_globally_enabled() {
                continue;
            }

            let distance_to_camera = (light.global_position() - camera.global_position()).norm();

            let (raw_radius, shadows_distance, shadows_enabled, shadows_fade_out_range) =
                if let Some(spot_light) = light.cast::<SpotLight>() {
                    (
                        spot_light.distance(),
                        settings.spot_shadows_distance,
                        spot_light.base_light_ref().is_cast_shadows()
                            && distance_to_camera <= settings.spot_shadows_distance
                            && settings.spot_shadows_enabled,
                        settings.spot_shadows_fade_out_range,
                    )
                } else if let Some(point_light) = light.cast::<PointLight>() {
                    (
                        point_light.radius(),
                        settings.point_shadows_distance,
                        point_light.base_light_ref().is_cast_shadows()
                            && distance_to_camera <= settings.point_shadows_distance
                            && settings.point_shadows_enabled,
                        settings.point_shadows_fade_out_range,
                    )
                } else if let Some(directional) = light.cast::<DirectionalLight>() {
                    (
                        f32::MAX,
                        0.0,
                        directional.base_light_ref().is_cast_shadows()
                            && settings.csm_settings.enabled,
                        0.0,
                    )
                } else {
                    continue;
                };

            let light_position = light.global_position();
            let scl = light.local_transform().scale();
            let light_radius_scale = scl.x.max(scl.y).max(scl.z);
            let light_radius = light_radius_scale * raw_radius;
            let light_r_inflate = 1.05 * light_radius;
            let light_radius_vec = Vector3::new(light_r_inflate, light_r_inflate, light_r_inflate);
            let emit_direction = light
                .up_vector()
                .try_normalize(f32::EPSILON)
                .unwrap_or_else(Vector3::z);

            if !frustum.is_intersects_sphere(light_position, light_radius) {
                continue;
            }

            let b1 = shadows_distance * 0.2;
            let b2 = shadows_distance * 0.4;
            let cascade_index =
                if distance_to_camera < b1 || frustum.is_contains_point(camera.global_position()) {
                    0
                } else if distance_to_camera > b1 && distance_to_camera < b2 {
                    1
                } else {
                    2
                };

            let left_boundary = (shadows_distance - shadows_fade_out_range).max(0.0);
            let shadows_alpha = if distance_to_camera <= left_boundary {
                1.0
            } else {
                1.0 - (distance_to_camera - left_boundary) / shadows_fade_out_range
            };

            let mut light_view_projection = Matrix4::identity();

            if shadows_enabled {
                if let Some(spot) = light.cast::<SpotLight>() {
                    let z_near = 0.01;
                    let z_far = light_radius;
                    let light_projection_matrix =
                        Matrix4::new_perspective(1.0, spot.full_cone_angle(), z_near, z_far);

                    let light_look_at = light_position - emit_direction;

                    let light_up_vec = light
                        .look_vector()
                        .try_normalize(f32::EPSILON)
                        .unwrap_or_else(Vector3::y);

                    let light_view_matrix = Matrix4::look_at_rh(
                        &Point3::from(light_position),
                        &Point3::from(light_look_at),
                        &light_up_vec,
                    );

                    light_view_projection = light_projection_matrix * light_view_matrix;

                    pass_stats += self.spot_shadow_map_renderer.render(
                        state,
                        &scene.graph,
                        light_position,
                        light_view_matrix,
                        z_near,
                        z_far,
                        light_projection_matrix,
                        geometry_cache,
                        cascade_index,
                        shader_cache,
                        textures,
                        normal_dummy.clone(),
                        white_dummy.clone(),
                        black_dummy.clone(),
                        volume_dummy.clone(),
                        matrix_storage,
                    )?;

                    light_stats.spot_shadow_maps_rendered += 1;
                } else if light.cast::<PointLight>().is_some() {
                    pass_stats +=
                        self.point_shadow_map_renderer
                            .render(PointShadowMapRenderContext {
                                state,
                                graph: &scene.graph,
                                light_pos: light_position,
                                light_radius,
                                geom_cache: geometry_cache,
                                cascade: cascade_index,
                                shader_cache,
                                texture_cache: textures,
                                normal_dummy: normal_dummy.clone(),
                                white_dummy: white_dummy.clone(),
                                black_dummy: black_dummy.clone(),
                                volume_dummy: volume_dummy.clone(),
                                matrix_storage,
                            })?;

                    light_stats.point_shadow_maps_rendered += 1;
                } else if let Some(directional) = light.cast::<DirectionalLight>() {
                    pass_stats += self.csm_renderer.render(CsmRenderContext {
                        frame_size: Vector2::new(gbuffer.width as f32, gbuffer.height as f32),
                        state,
                        graph: &scene.graph,
                        light: directional,
                        camera,
                        geom_cache: geometry_cache,
                        shader_cache,
                        texture_cache: textures,
                        normal_dummy: normal_dummy.clone(),
                        white_dummy: white_dummy.clone(),
                        black_dummy: black_dummy.clone(),
                        volume_dummy: volume_dummy.clone(),
                        matrix_storage,
                    })?;

                    light_stats.csm_rendered += 1;
                };
            }

            // Mark lighted areas in stencil buffer to do light calculations only on them.

            let sphere = &self.sphere;

            pass_stats += frame_buffer.draw(
                sphere,
                state,
                viewport,
                &self.flat_shader.program,
                &DrawParameters {
                    cull_face: Some(CullFace::Front),
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Always,
                        ..Default::default()
                    }),
                    stencil_op: StencilOp {
                        zfail: StencilAction::Incr,
                        ..Default::default()
                    },
                    depth_test: true,
                    blend: None,
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding.set_matrix4(
                        &self.flat_shader.wvp_matrix,
                        &(view_projection
                            * Matrix4::new_translation(&light_position)
                            * Matrix4::new_nonuniform_scaling(&light_radius_vec)),
                    );
                },
            )?;

            pass_stats += frame_buffer.draw(
                sphere,
                state,
                viewport,
                &self.flat_shader.program,
                &DrawParameters {
                    cull_face: Some(CullFace::Back),
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Always,
                        ..Default::default()
                    }),
                    stencil_op: StencilOp {
                        zfail: StencilAction::Decr,
                        ..Default::default()
                    },
                    depth_test: true,
                    blend: None,
                },
                ElementRange::Full,
                |mut program_binding| {
                    program_binding.set_matrix4(
                        &self.flat_shader.wvp_matrix,
                        &(view_projection
                            * Matrix4::new_translation(&light_position)
                            * Matrix4::new_nonuniform_scaling(&light_radius_vec)),
                    );
                },
            )?;

            let draw_params = DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: Some(StencilFunc {
                    func: CompareFunc::NotEqual,
                    ..Default::default()
                }),
                stencil_op: StencilOp {
                    zpass: StencilAction::Zero,
                    ..Default::default()
                },
                depth_test: false,
                blend: Some(BlendParameters {
                    func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                    ..Default::default()
                }),
            };

            let quad = &self.quad;

            pass_stats += if let Some(spot_light) = light.cast::<SpotLight>() {
                let shader = &self.spot_light_shader;

                let (cookie_enabled, cookie_texture) =
                    if let Some(texture) = spot_light.cookie_texture_ref() {
                        if let Some(cookie) = textures.get(state, texture) {
                            (true, cookie)
                        } else {
                            (false, &white_dummy)
                        }
                    } else {
                        (false, &white_dummy)
                    };

                light_stats.spot_lights_rendered += 1;

                frame_buffer.draw(
                    quad,
                    state,
                    viewport,
                    &shader.program,
                    &draw_params,
                    ElementRange::Full,
                    |mut program_binding| {
                        program_binding
                            .set_bool(&shader.shadows_enabled, shadows_enabled)
                            .set_matrix4(&shader.light_view_proj_matrix, &light_view_projection)
                            .set_bool(&shader.soft_shadows, settings.spot_soft_shadows)
                            .set_vector3(&shader.light_position, &light_position)
                            .set_vector3(&shader.light_direction, &emit_direction)
                            .set_f32(&shader.light_radius, light_radius)
                            .set_matrix4(&shader.inv_view_proj_matrix, &inv_view_projection)
                            .set_linear_color(
                                &shader.light_color,
                                &spot_light.base_light_ref().color(),
                            )
                            .set_f32(
                                &shader.half_hotspot_cone_angle_cos,
                                (spot_light.hotspot_cone_angle() * 0.5).cos(),
                            )
                            .set_f32(
                                &shader.half_cone_angle_cos,
                                (spot_light.full_cone_angle() * 0.5).cos(),
                            )
                            .set_matrix4(&shader.wvp_matrix, &frame_matrix)
                            .set_f32(
                                &shader.shadow_map_inv_size,
                                1.0 / (self.spot_shadow_map_renderer.cascade_size(cascade_index)
                                    as f32),
                            )
                            .set_vector3(&shader.camera_position, &camera_global_position)
                            .set_texture(&shader.depth_sampler, &gbuffer_depth_map)
                            .set_texture(&shader.color_sampler, &gbuffer_diffuse_map)
                            .set_texture(&shader.normal_sampler, &gbuffer_normal_map)
                            .set_texture(&shader.material_sampler, &gbuffer_material_map)
                            .set_texture(
                                &shader.spot_shadow_texture,
                                &self.spot_shadow_map_renderer.cascade_texture(cascade_index),
                            )
                            .set_texture(&shader.cookie_texture, cookie_texture)
                            .set_bool(&shader.cookie_enabled, cookie_enabled)
                            .set_f32(&shader.shadow_bias, spot_light.shadow_bias())
                            .set_f32(
                                &shader.light_intensity,
                                spot_light.base_light_ref().intensity(),
                            )
                            .set_f32(&shader.shadow_alpha, shadows_alpha);
                    },
                )?
            } else if let Some(point_light) = light.cast::<PointLight>() {
                let shader = &self.point_light_shader;

                light_stats.point_lights_rendered += 1;

                frame_buffer.draw(
                    quad,
                    state,
                    viewport,
                    &shader.program,
                    &draw_params,
                    ElementRange::Full,
                    |mut program_binding| {
                        program_binding
                            .set_bool(&shader.shadows_enabled, shadows_enabled)
                            .set_bool(&shader.soft_shadows, settings.point_soft_shadows)
                            .set_vector3(&shader.light_position, &light_position)
                            .set_f32(&shader.light_radius, light_radius)
                            .set_matrix4(&shader.inv_view_proj_matrix, &inv_view_projection)
                            .set_linear_color(
                                &shader.light_color,
                                &point_light.base_light_ref().color(),
                            )
                            .set_matrix4(&shader.wvp_matrix, &frame_matrix)
                            .set_vector3(&shader.camera_position, &camera_global_position)
                            .set_f32(&shader.shadow_bias, point_light.shadow_bias())
                            .set_f32(
                                &shader.light_intensity,
                                point_light.base_light_ref().intensity(),
                            )
                            .set_texture(&shader.depth_sampler, &gbuffer_depth_map)
                            .set_texture(&shader.color_sampler, &gbuffer_diffuse_map)
                            .set_texture(&shader.normal_sampler, &gbuffer_normal_map)
                            .set_texture(&shader.material_sampler, &gbuffer_material_map)
                            .set_texture(
                                &shader.point_shadow_texture,
                                &self
                                    .point_shadow_map_renderer
                                    .cascade_texture(cascade_index),
                            )
                            .set_f32(&shader.shadow_alpha, shadows_alpha);
                    },
                )?
            } else if let Some(directional) = light.cast::<DirectionalLight>() {
                let shader = &self.directional_light_shader;

                light_stats.directional_lights_rendered += 1;

                frame_buffer.draw(
                    quad,
                    state,
                    viewport,
                    &shader.program,
                    &DrawParameters {
                        cull_face: None,
                        color_write: Default::default(),
                        depth_write: false,
                        stencil_test: None,
                        depth_test: false,
                        blend: Some(BlendParameters {
                            func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                            ..Default::default()
                        }),
                        stencil_op: Default::default(),
                    },
                    ElementRange::Full,
                    |mut program_binding| {
                        let distances = [
                            self.csm_renderer.cascades()[0].z_far,
                            self.csm_renderer.cascades()[1].z_far,
                            self.csm_renderer.cascades()[2].z_far,
                        ];
                        let matrices = [
                            self.csm_renderer.cascades()[0].view_proj_matrix,
                            self.csm_renderer.cascades()[1].view_proj_matrix,
                            self.csm_renderer.cascades()[2].view_proj_matrix,
                        ];
                        let csm_map_size = self.csm_renderer.size() as f32;

                        program_binding
                            .set_vector3(&shader.light_direction, &emit_direction)
                            .set_matrix4(&shader.inv_view_proj_matrix, &inv_view_projection)
                            .set_linear_color(
                                &shader.light_color,
                                &directional.base_light_ref().color(),
                            )
                            .set_matrix4(&shader.wvp_matrix, &frame_matrix)
                            .set_vector3(&shader.camera_position, &camera_global_position)
                            .set_f32(
                                &shader.light_intensity,
                                directional.base_light_ref().intensity(),
                            )
                            .set_texture(&shader.depth_sampler, &gbuffer_depth_map)
                            .set_texture(&shader.color_sampler, &gbuffer_diffuse_map)
                            .set_texture(&shader.normal_sampler, &gbuffer_normal_map)
                            .set_texture(&shader.material_sampler, &gbuffer_material_map)
                            .set_matrix4_array(&shader.light_view_proj_matrices, &matrices)
                            .set_texture(
                                &shader.shadow_cascade0,
                                &self.csm_renderer.cascades()[0].texture(),
                            )
                            .set_texture(
                                &shader.shadow_cascade1,
                                &self.csm_renderer.cascades()[1].texture(),
                            )
                            .set_texture(
                                &shader.shadow_cascade2,
                                &self.csm_renderer.cascades()[2].texture(),
                            )
                            .set_f32_slice(&shader.cascade_distances, &distances)
                            .set_matrix4(&shader.view_matrix, &camera.view_matrix())
                            .set_f32(&shader.shadow_bias, directional.csm_options.shadow_bias())
                            .set_bool(&shader.shadows_enabled, shadows_enabled)
                            .set_bool(&shader.soft_shadows, settings.csm_settings.pcf)
                            .set_f32(&shader.shadow_map_inv_size, 1.0 / csm_map_size);
                    },
                )?
            } else {
                unreachable!()
            };

            if settings.light_scatter_enabled {
                pass_stats += self.light_volume.render_volume(
                    state,
                    light,
                    light_handle,
                    gbuffer,
                    &self.quad,
                    camera.view_matrix(),
                    inv_projection,
                    view_projection,
                    viewport,
                    &scene.graph,
                    frame_buffer,
                )?;
            }
        }

        Ok((pass_stats, light_stats))
    }
}
