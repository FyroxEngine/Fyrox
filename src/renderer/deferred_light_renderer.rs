use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        color::Color,
        math::{frustum::Frustum, Matrix4Ext, Rect, TriangleDefinition},
        scope_profile,
    },
    renderer::{
        batch::BatchStorage,
        error::RendererError,
        flat_shader::FlatShader,
        framework::{
            framebuffer::{CullFace, DrawParameters, DrawPartContext, FrameBufferTrait},
            gl,
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::GpuTexture,
            state::{ColorMask, PipelineState, StencilFunc, StencilOp},
        },
        gbuffer::GBuffer,
        light_volume::LightVolumeRenderer,
        shadow_map_renderer::{
            PointShadowMapRenderContext, PointShadowMapRenderer, SpotShadowMapRenderer,
        },
        ssao::ScreenSpaceAmbientOcclusionRenderer,
        surface::{SurfaceSharedData, Vertex},
        GeometryCache, QualitySettings, RenderPassStatistics, TextureCache,
    },
    scene::{camera::Camera, light::Light, node::Node, Scene},
};
use std::{
    cell::RefCell,
    fmt::{Display, Formatter},
    ops::AddAssign,
    rc::Rc,
};

struct AmbientLightShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
    ambient_color: UniformLocation,
    ao_sampler: UniformLocation,
    ambient_texture: UniformLocation,
}

#[derive(Copy, Clone, Default)]
pub struct LightingStatistics {
    pub point_lights_rendered: usize,
    pub point_shadow_maps_rendered: usize,
    pub spot_lights_rendered: usize,
    pub spot_shadow_maps_rendered: usize,
    pub directional_lights_rendered: usize,
}

impl AddAssign for LightingStatistics {
    fn add_assign(&mut self, rhs: Self) {
        self.point_lights_rendered += rhs.point_lights_rendered;
        self.point_shadow_maps_rendered += rhs.point_shadow_maps_rendered;
        self.spot_lights_rendered += rhs.spot_lights_rendered;
        self.spot_shadow_maps_rendered += rhs.spot_shadow_maps_rendered;
        self.directional_lights_rendered += rhs.directional_lights_rendered;
    }
}

impl Display for LightingStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Lighting Statistics:\n\
            \tPoint Lights: {}\n\
            \tSpot Lights: {}\n\
            \tDirectional Lights: {}\n\
            \tPoint Shadow Maps: {}\n\
            \tSpot Shadow Maps: {}",
            self.point_lights_rendered,
            self.spot_lights_rendered,
            self.directional_lights_rendered,
            self.point_shadow_maps_rendered,
            self.spot_shadow_maps_rendered,
        )
    }
}

impl AmbientLightShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/ambient_light_fs.glsl");
        let vertex_source = include_str!("shaders/ambient_light_vs.glsl");
        let program =
            GpuProgram::from_source("AmbientLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            ambient_color: program.uniform_location("ambientColor")?,
            ao_sampler: program.uniform_location("aoSampler")?,
            ambient_texture: program.uniform_location("ambientTexture")?,
            program,
        })
    }
}

struct SpotLightShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    depth_sampler: UniformLocation,
    color_sampler: UniformLocation,
    normal_sampler: UniformLocation,
    spot_shadow_texture: UniformLocation,
    cookie_enabled: UniformLocation,
    cookie_texture: UniformLocation,
    light_view_proj_matrix: UniformLocation,
    shadows_enabled: UniformLocation,
    soft_shadows: UniformLocation,
    shadow_map_inv_size: UniformLocation,
    light_position: UniformLocation,
    light_radius: UniformLocation,
    light_color: UniformLocation,
    light_direction: UniformLocation,
    half_hotspot_cone_angle_cos: UniformLocation,
    half_cone_angle_cos: UniformLocation,
    inv_view_proj_matrix: UniformLocation,
    camera_position: UniformLocation,
    shadow_bias: UniformLocation,
}

impl SpotLightShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/deferred_spot_light_fs.glsl");
        let vertex_source = include_str!("shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source("DeferredLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            depth_sampler: program.uniform_location("depthTexture")?,
            color_sampler: program.uniform_location("colorTexture")?,
            normal_sampler: program.uniform_location("normalTexture")?,
            spot_shadow_texture: program.uniform_location("spotShadowTexture")?,
            cookie_enabled: program.uniform_location("cookieEnabled")?,
            cookie_texture: program.uniform_location("cookieTexture")?,
            light_view_proj_matrix: program.uniform_location("lightViewProjMatrix")?,
            shadows_enabled: program.uniform_location("shadowsEnabled")?,
            soft_shadows: program.uniform_location("softShadows")?,
            shadow_map_inv_size: program.uniform_location("shadowMapInvSize")?,
            light_position: program.uniform_location("lightPos")?,
            light_radius: program.uniform_location("lightRadius")?,
            light_color: program.uniform_location("lightColor")?,
            light_direction: program.uniform_location("lightDirection")?,
            half_hotspot_cone_angle_cos: program.uniform_location("halfHotspotConeAngleCos")?,
            half_cone_angle_cos: program.uniform_location("halfConeAngleCos")?,
            inv_view_proj_matrix: program.uniform_location("invViewProj")?,
            camera_position: program.uniform_location("cameraPosition")?,
            shadow_bias: program.uniform_location("shadowBias")?,

            program,
        })
    }
}

struct PointLightShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    depth_sampler: UniformLocation,
    color_sampler: UniformLocation,
    normal_sampler: UniformLocation,
    point_shadow_texture: UniformLocation,
    shadows_enabled: UniformLocation,
    soft_shadows: UniformLocation,
    light_position: UniformLocation,
    light_radius: UniformLocation,
    light_color: UniformLocation,
    inv_view_proj_matrix: UniformLocation,
    camera_position: UniformLocation,
    shadow_bias: UniformLocation,
}

impl PointLightShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/deferred_point_light_fs.glsl");
        let vertex_source = include_str!("shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source("DeferredLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            depth_sampler: program.uniform_location("depthTexture")?,
            color_sampler: program.uniform_location("colorTexture")?,
            normal_sampler: program.uniform_location("normalTexture")?,
            point_shadow_texture: program.uniform_location("pointShadowTexture")?,
            shadows_enabled: program.uniform_location("shadowsEnabled")?,
            soft_shadows: program.uniform_location("softShadows")?,
            light_position: program.uniform_location("lightPos")?,
            light_radius: program.uniform_location("lightRadius")?,
            light_color: program.uniform_location("lightColor")?,
            inv_view_proj_matrix: program.uniform_location("invViewProj")?,
            camera_position: program.uniform_location("cameraPosition")?,
            shadow_bias: program.uniform_location("shadowBias")?,

            program,
        })
    }
}

struct DirectionalLightShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    depth_sampler: UniformLocation,
    color_sampler: UniformLocation,
    normal_sampler: UniformLocation,
    light_direction: UniformLocation,
    light_color: UniformLocation,
    inv_view_proj_matrix: UniformLocation,
    camera_position: UniformLocation,
}

impl DirectionalLightShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/deferred_directional_light_fs.glsl");
        let vertex_source = include_str!("shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source("DeferredLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            depth_sampler: program.uniform_location("depthTexture")?,
            color_sampler: program.uniform_location("colorTexture")?,
            normal_sampler: program.uniform_location("normalTexture")?,
            light_direction: program.uniform_location("lightDirection")?,
            light_color: program.uniform_location("lightColor")?,
            inv_view_proj_matrix: program.uniform_location("invViewProj")?,
            camera_position: program.uniform_location("cameraPosition")?,
            program,
        })
    }
}

pub struct DeferredLightRenderer {
    pub ssao_renderer: ScreenSpaceAmbientOcclusionRenderer,
    spot_light_shader: SpotLightShader,
    point_light_shader: PointLightShader,
    directional_light_shader: DirectionalLightShader,
    ambient_light_shader: AmbientLightShader,
    quad: SurfaceSharedData,
    sphere: SurfaceSharedData,
    skybox: SurfaceSharedData,
    flat_shader: FlatShader,
    spot_shadow_map_renderer: SpotShadowMapRenderer,
    point_shadow_map_renderer: PointShadowMapRenderer,
    light_volume: LightVolumeRenderer,
}

pub(in crate) struct DeferredRendererContext<'a> {
    pub state: &'a mut PipelineState,
    pub scene: &'a Scene,
    pub camera: &'a Camera,
    pub gbuffer: &'a mut GBuffer,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub ambient_color: Color,
    pub settings: &'a QualitySettings,
    pub textures: &'a mut TextureCache,
    pub geometry_cache: &'a mut GeometryCache,
    pub batch_storage: &'a BatchStorage,
}

impl DeferredLightRenderer {
    pub fn new(
        state: &mut PipelineState,
        frame_size: (u32, u32),
        settings: &QualitySettings,
    ) -> Result<Self, RendererError> {
        Ok(Self {
            ssao_renderer: ScreenSpaceAmbientOcclusionRenderer::new(
                state,
                frame_size.0 as usize,
                frame_size.1 as usize,
            )?,
            spot_light_shader: SpotLightShader::new()?,
            point_light_shader: PointLightShader::new()?,
            directional_light_shader: DirectionalLightShader::new()?,
            ambient_light_shader: AmbientLightShader::new()?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
            skybox: SurfaceSharedData::new(
                vec![
                    // Front
                    Vertex::from_pos_uv(Vector3::new(-0.5, 0.5, -0.5), Vector2::new(0.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, 0.5, -0.5), Vector2::new(1.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, -0.5, -0.5), Vector2::new(1.0, 1.0)),
                    Vertex::from_pos_uv(Vector3::new(-0.5, -0.5, -0.5), Vector2::new(0.0, 1.0)),
                    // Back
                    Vertex::from_pos_uv(Vector3::new(0.5, 0.5, 0.5), Vector2::new(0.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(-0.5, 0.5, 0.5), Vector2::new(1.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(-0.5, -0.5, 0.5), Vector2::new(1.0, 1.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, -0.5, 0.5), Vector2::new(0.0, 1.0)),
                    // Left
                    Vertex::from_pos_uv(Vector3::new(0.5, 0.5, -0.5), Vector2::new(0.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, 0.5, 0.5), Vector2::new(1.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, -0.5, 0.5), Vector2::new(1.0, 1.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, -0.5, -0.5), Vector2::new(0.0, 1.0)),
                    // Right
                    Vertex::from_pos_uv(Vector3::new(-0.5, 0.5, 0.5), Vector2::new(0.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(-0.5, 0.5, -0.5), Vector2::new(1.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(-0.5, -0.5, -0.5), Vector2::new(1.0, 1.0)),
                    Vertex::from_pos_uv(Vector3::new(-0.5, -0.5, 0.5), Vector2::new(0.0, 1.0)),
                    // Up
                    Vertex::from_pos_uv(Vector3::new(-0.5, 0.5, 0.5), Vector2::new(0.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, 0.5, 0.5), Vector2::new(1.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, 0.5, -0.5), Vector2::new(1.0, 1.0)),
                    Vertex::from_pos_uv(Vector3::new(-0.5, 0.5, -0.5), Vector2::new(0.0, 1.0)),
                    // Down
                    Vertex::from_pos_uv(Vector3::new(-0.5, -0.5, 0.5), Vector2::new(0.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, -0.5, 0.5), Vector2::new(1.0, 0.0)),
                    Vertex::from_pos_uv(Vector3::new(0.5, -0.5, -0.5), Vector2::new(1.0, 1.0)),
                    Vertex::from_pos_uv(Vector3::new(-0.5, -0.5, -0.5), Vector2::new(0.0, 1.0)),
                ],
                vec![
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
                ],
                true,
            ),
            sphere: SurfaceSharedData::make_sphere(6, 6, 1.0),
            flat_shader: FlatShader::new()?,
            spot_shadow_map_renderer: SpotShadowMapRenderer::new(
                state,
                settings.spot_shadow_map_size,
                QualitySettings::default().spot_shadow_map_precision,
            )?,
            point_shadow_map_renderer: PointShadowMapRenderer::new(
                state,
                settings.point_shadow_map_size,
                QualitySettings::default().point_shadow_map_precision,
            )?,
            light_volume: LightVolumeRenderer::new()?,
        })
    }

    pub fn set_quality_settings(
        &mut self,
        state: &mut PipelineState,
        settings: &QualitySettings,
    ) -> Result<(), RendererError> {
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
        self.ssao_renderer.set_radius(settings.ssao_radius);
        Ok(())
    }

    pub fn set_frame_size(
        &mut self,
        state: &mut PipelineState,
        frame_size: (u32, u32),
    ) -> Result<(), RendererError> {
        self.ssao_renderer = ScreenSpaceAmbientOcclusionRenderer::new(
            state,
            frame_size.0 as usize,
            frame_size.1 as usize,
        )?;
        Ok(())
    }

    #[must_use]
    pub(in crate) fn render(
        &mut self,
        args: DeferredRendererContext,
    ) -> (RenderPassStatistics, LightingStatistics) {
        scope_profile!();

        let mut pass_stats = RenderPassStatistics::default();
        let mut light_stats = LightingStatistics::default();

        let DeferredRendererContext {
            state,
            scene,
            camera,
            gbuffer,
            white_dummy,
            ambient_color,
            settings,
            textures,
            geometry_cache,
            batch_storage,
        } = args;

        let viewport = Rect::new(0, 0, gbuffer.width, gbuffer.height);
        let frustum = Frustum::from(camera.view_projection_matrix()).unwrap();

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
        let inv_view_projection = view_projection.try_inverse().unwrap_or_default();

        // Fill SSAO map.
        if settings.use_ssao {
            pass_stats += self.ssao_renderer.render(
                state,
                gbuffer,
                geometry_cache,
                projection_matrix,
                camera.view_matrix().basis(),
            );
        }

        gbuffer.final_frame.clear(
            state,
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            None,
            Some(0),
        );

        // Render skybox (if any).
        if let Some(skybox) = camera.skybox_ref() {
            let size = camera.z_far() / 2.0f32.sqrt();
            let scale = Matrix4::new_nonuniform_scaling(&Vector3::new(size, size, size));
            let wvp = Matrix4::new_translation(&camera.global_position()) * scale;

            // TODO: Ideally this should be drawn in a single draw call using cube map.
            // Cubemaps still not supported so we'll draw this as six separate planes for now.
            for (face, texture) in skybox
                .textures()
                .iter()
                .enumerate()
                .filter_map(|(face, tex)| tex.clone().map(|tex| (face, tex)))
            {
                if let Some(gpu_texture) = textures.get(state, texture) {
                    pass_stats += gbuffer
                        .final_frame
                        .draw_part(DrawPartContext {
                            geometry: geometry_cache.get(state, &self.skybox),
                            state,
                            viewport,
                            program: &mut self.flat_shader.program,
                            params: DrawParameters {
                                cull_face: CullFace::Back,
                                culling: false,
                                color_write: Default::default(),
                                depth_write: false,
                                stencil_test: false,
                                depth_test: false,
                                blend: false,
                            },
                            uniforms: &[
                                (
                                    self.flat_shader.diffuse_texture,
                                    UniformValue::Sampler {
                                        index: 0,
                                        texture: gpu_texture,
                                    },
                                ),
                                (
                                    self.flat_shader.wvp_matrix,
                                    UniformValue::Matrix4(view_projection * wvp),
                                ),
                            ],
                            offset: face * 2,
                            count: 2,
                        })
                        .unwrap();
                }
            }
        }

        state.set_blend(true);
        state.set_blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        // Ambient light.
        gbuffer.final_frame.draw(
            geometry_cache.get(state, &self.quad),
            state,
            viewport,
            &self.ambient_light_shader.program,
            &DrawParameters {
                cull_face: CullFace::Back,
                culling: false,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: false,
                depth_test: false,
                blend: true,
            },
            &[
                (
                    self.ambient_light_shader.wvp_matrix,
                    UniformValue::Matrix4(frame_matrix),
                ),
                (
                    self.ambient_light_shader.ambient_color,
                    UniformValue::Color(ambient_color),
                ),
                (
                    self.ambient_light_shader.diffuse_texture,
                    UniformValue::Sampler {
                        index: 0,
                        texture: gbuffer.diffuse_texture(),
                    },
                ),
                (
                    self.ambient_light_shader.ao_sampler,
                    UniformValue::Sampler {
                        index: 1,
                        texture: if settings.use_ssao {
                            self.ssao_renderer.ao_map()
                        } else {
                            white_dummy.clone()
                        },
                    },
                ),
                (
                    self.ambient_light_shader.ambient_texture,
                    UniformValue::Sampler {
                        index: 2,
                        texture: gbuffer.ambient_texture(),
                    },
                ),
            ],
        );

        state.set_blend_func(gl::ONE, gl::ONE);

        for (light_handle, light) in scene.graph.pair_iter().filter_map(|(handle, node)| {
            if let Node::Light(light) = node {
                Some((handle, light))
            } else {
                None
            }
        }) {
            if !light.global_visibility() {
                continue;
            }

            let raw_radius = match light {
                Light::Spot(spot_light) => spot_light.distance(),
                Light::Point(point_light) => point_light.radius(),
                Light::Directional(_) => std::f32::MAX,
            };

            let light_position = light.global_position();
            let scl = light.local_transform().scale();
            let light_radius_scale = scl.x.max(scl.y).max(scl.z);
            let light_radius = light_radius_scale * raw_radius;
            let light_r_inflate = 1.05 * light_radius;
            let light_radius_vec = Vector3::new(light_r_inflate, light_r_inflate, light_r_inflate);
            let emit_direction = light
                .up_vector()
                .try_normalize(std::f32::EPSILON)
                .unwrap_or_else(Vector3::z);

            if !frustum.is_intersects_sphere(light_position, light_radius) {
                continue;
            }

            let distance_to_camera = (light.global_position() - camera.global_position()).norm();

            let v = match light {
                Light::Directional(_) => 0.0,
                Light::Spot(_) => settings.spot_shadows_distance,
                Light::Point(_) => settings.point_shadows_distance,
            };
            let b1 = v * 0.2;
            let b2 = v * 0.4;
            let cascade_index = if distance_to_camera < b1 {
                0
            } else if distance_to_camera > b1 && distance_to_camera < b2 {
                1
            } else {
                2
            };

            let mut light_view_projection = Matrix4::identity();
            let shadows_enabled = light.is_cast_shadows()
                && match light {
                    Light::Spot(spot)
                        if distance_to_camera <= settings.spot_shadows_distance
                            && settings.spot_shadows_enabled =>
                    {
                        let light_projection_matrix = Matrix4::new_perspective(
                            1.0,
                            spot.full_cone_angle(),
                            0.01,
                            light_radius,
                        );

                        let light_look_at = light_position - emit_direction;

                        let light_up_vec = light
                            .look_vector()
                            .try_normalize(std::f32::EPSILON)
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
                            &light_view_projection,
                            batch_storage,
                            geometry_cache,
                            cascade_index,
                        );

                        light_stats.spot_shadow_maps_rendered += 1;

                        true
                    }
                    Light::Point(_)
                        if distance_to_camera <= settings.point_shadows_distance
                            && settings.point_shadows_enabled =>
                    {
                        pass_stats +=
                            self.point_shadow_map_renderer
                                .render(PointShadowMapRenderContext {
                                    state,
                                    graph: &scene.graph,
                                    light_pos: light_position,
                                    light_radius,
                                    geom_cache: geometry_cache,
                                    cascade: cascade_index,
                                    batch_storage,
                                });

                        light_stats.point_shadow_maps_rendered += 1;

                        true
                    }
                    Light::Directional(_) => {
                        // TODO: Add cascaded shadow map.
                        false
                    }
                    _ => false,
                };

            // Mark lighted areas in stencil buffer to do light calculations only on them.
            state.set_stencil_mask(0xFFFF_FFFF);
            state.set_stencil_func(StencilFunc {
                func: gl::ALWAYS,
                ..Default::default()
            });
            state.set_stencil_op(StencilOp {
                zfail: gl::INCR,
                ..Default::default()
            });

            let sphere = geometry_cache.get(state, &self.sphere);

            pass_stats += gbuffer.final_frame.draw(
                sphere,
                state,
                viewport,
                &self.flat_shader.program,
                &DrawParameters {
                    cull_face: CullFace::Front,
                    culling: true,
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: true,
                    depth_test: true,
                    blend: false,
                },
                &[(
                    self.flat_shader.wvp_matrix,
                    UniformValue::Matrix4(
                        view_projection
                            * Matrix4::new_translation(&light_position)
                            * Matrix4::new_nonuniform_scaling(&light_radius_vec),
                    ),
                )],
            );

            state.set_stencil_func(StencilFunc {
                func: gl::ALWAYS,
                ..Default::default()
            });
            state.set_stencil_op(StencilOp {
                zfail: gl::DECR,
                ..Default::default()
            });

            pass_stats += gbuffer.final_frame.draw(
                sphere,
                state,
                viewport,
                &self.flat_shader.program,
                &DrawParameters {
                    cull_face: CullFace::Back,
                    culling: true,
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: true,
                    depth_test: true,
                    blend: false,
                },
                &[(
                    self.flat_shader.wvp_matrix,
                    UniformValue::Matrix4(
                        view_projection
                            * Matrix4::new_translation(&light_position)
                            * Matrix4::new_nonuniform_scaling(&light_radius_vec),
                    ),
                )],
            );

            state.set_stencil_func(StencilFunc {
                func: gl::NOTEQUAL,
                ..Default::default()
            });
            state.set_stencil_op(StencilOp {
                zpass: gl::ZERO,
                ..Default::default()
            });

            let draw_params = DrawParameters {
                cull_face: CullFace::Back,
                culling: false,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: true,
                depth_test: false,
                blend: true,
            };

            let quad = geometry_cache.get(state, &self.quad);

            pass_stats += match light {
                Light::Spot(spot_light) => {
                    let shader = &self.spot_light_shader;

                    let (cookie_enabled, cookie_texture) =
                        if let Some(texture) = spot_light.cookie_texture() {
                            (true, textures.get(state, texture.clone()).unwrap())
                        } else {
                            (false, white_dummy.clone())
                        };

                    let uniforms = [
                        (shader.shadows_enabled, UniformValue::Bool(shadows_enabled)),
                        (
                            shader.light_view_proj_matrix,
                            UniformValue::Matrix4(light_view_projection),
                        ),
                        (
                            shader.soft_shadows,
                            UniformValue::Bool(settings.spot_soft_shadows),
                        ),
                        (shader.light_position, UniformValue::Vector3(light_position)),
                        (
                            shader.light_direction,
                            UniformValue::Vector3(emit_direction),
                        ),
                        (shader.light_radius, UniformValue::Float(light_radius)),
                        (
                            shader.inv_view_proj_matrix,
                            UniformValue::Matrix4(inv_view_projection),
                        ),
                        (shader.light_color, UniformValue::Color(light.color())),
                        (
                            shader.half_hotspot_cone_angle_cos,
                            UniformValue::Float((spot_light.hotspot_cone_angle() * 0.5).cos()),
                        ),
                        (
                            shader.half_cone_angle_cos,
                            UniformValue::Float((spot_light.full_cone_angle() * 0.5).cos()),
                        ),
                        (shader.wvp_matrix, UniformValue::Matrix4(frame_matrix)),
                        (
                            shader.shadow_map_inv_size,
                            UniformValue::Float(
                                1.0 / (self.spot_shadow_map_renderer.cascade_size(cascade_index)
                                    as f32),
                            ),
                        ),
                        (
                            shader.camera_position,
                            UniformValue::Vector3(camera.global_position()),
                        ),
                        (
                            shader.depth_sampler,
                            UniformValue::Sampler {
                                index: 0,
                                texture: gbuffer.depth(),
                            },
                        ),
                        (
                            shader.color_sampler,
                            UniformValue::Sampler {
                                index: 1,
                                texture: gbuffer.diffuse_texture(),
                            },
                        ),
                        (
                            shader.normal_sampler,
                            UniformValue::Sampler {
                                index: 2,
                                texture: gbuffer.normal_texture(),
                            },
                        ),
                        (
                            shader.spot_shadow_texture,
                            UniformValue::Sampler {
                                index: 3,
                                texture: self
                                    .spot_shadow_map_renderer
                                    .cascade_texture(cascade_index),
                            },
                        ),
                        (shader.cookie_enabled, UniformValue::Bool(cookie_enabled)),
                        (
                            shader.cookie_texture,
                            UniformValue::Sampler {
                                index: 4,
                                texture: cookie_texture,
                            },
                        ),
                        (
                            shader.shadow_bias,
                            UniformValue::Float(spot_light.shadow_bias()),
                        ),
                    ];

                    light_stats.spot_lights_rendered += 1;

                    gbuffer.final_frame.draw(
                        quad,
                        state,
                        viewport,
                        &shader.program,
                        &draw_params,
                        &uniforms,
                    )
                }
                Light::Point(point_light) => {
                    let shader = &self.point_light_shader;

                    let uniforms = [
                        (shader.shadows_enabled, UniformValue::Bool(shadows_enabled)),
                        (
                            shader.soft_shadows,
                            UniformValue::Bool(settings.point_soft_shadows),
                        ),
                        (shader.light_position, UniformValue::Vector3(light_position)),
                        (shader.light_radius, UniformValue::Float(light_radius)),
                        (
                            shader.inv_view_proj_matrix,
                            UniformValue::Matrix4(inv_view_projection),
                        ),
                        (shader.light_color, UniformValue::Color(light.color())),
                        (shader.wvp_matrix, UniformValue::Matrix4(frame_matrix)),
                        (
                            shader.camera_position,
                            UniformValue::Vector3(camera.global_position()),
                        ),
                        (
                            shader.depth_sampler,
                            UniformValue::Sampler {
                                index: 0,
                                texture: gbuffer.depth(),
                            },
                        ),
                        (
                            shader.color_sampler,
                            UniformValue::Sampler {
                                index: 1,
                                texture: gbuffer.diffuse_texture(),
                            },
                        ),
                        (
                            shader.normal_sampler,
                            UniformValue::Sampler {
                                index: 2,
                                texture: gbuffer.normal_texture(),
                            },
                        ),
                        (
                            shader.point_shadow_texture,
                            UniformValue::Sampler {
                                index: 3,
                                texture: self
                                    .point_shadow_map_renderer
                                    .cascade_texture(cascade_index),
                            },
                        ),
                        (
                            shader.shadow_bias,
                            UniformValue::Float(point_light.shadow_bias()),
                        ),
                    ];

                    light_stats.point_lights_rendered += 1;

                    gbuffer.final_frame.draw(
                        quad,
                        state,
                        viewport,
                        &shader.program,
                        &draw_params,
                        &uniforms,
                    )
                }
                Light::Directional(_) => {
                    let shader = &self.directional_light_shader;

                    let uniforms = [
                        (
                            shader.light_direction,
                            UniformValue::Vector3(emit_direction),
                        ),
                        (
                            shader.inv_view_proj_matrix,
                            UniformValue::Matrix4(inv_view_projection),
                        ),
                        (shader.light_color, UniformValue::Color(light.color())),
                        (shader.wvp_matrix, UniformValue::Matrix4(frame_matrix)),
                        (
                            shader.camera_position,
                            UniformValue::Vector3(camera.global_position()),
                        ),
                        (
                            shader.depth_sampler,
                            UniformValue::Sampler {
                                index: 0,
                                texture: gbuffer.depth(),
                            },
                        ),
                        (
                            shader.color_sampler,
                            UniformValue::Sampler {
                                index: 1,
                                texture: gbuffer.diffuse_texture(),
                            },
                        ),
                        (
                            shader.normal_sampler,
                            UniformValue::Sampler {
                                index: 2,
                                texture: gbuffer.normal_texture(),
                            },
                        ),
                    ];

                    light_stats.directional_lights_rendered += 1;

                    gbuffer.final_frame.draw(
                        quad,
                        state,
                        viewport,
                        &shader.program,
                        &DrawParameters {
                            cull_face: CullFace::Back,
                            culling: false,
                            color_write: Default::default(),
                            depth_write: false,
                            stencil_test: false,
                            depth_test: false,
                            blend: true,
                        },
                        &uniforms,
                    )
                }
            };

            if settings.light_scatter_enabled {
                pass_stats += self.light_volume.render_volume(
                    state,
                    light,
                    light_handle,
                    gbuffer,
                    &self.quad,
                    geometry_cache,
                    camera.view_matrix(),
                    projection_matrix.try_inverse().unwrap_or_default(),
                    camera.view_projection_matrix(),
                    viewport,
                    &scene.graph,
                );
            }
        }

        (pass_stats, light_stats)
    }
}
