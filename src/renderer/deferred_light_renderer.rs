use crate::{
    core::{
        color::Color,
        math::{frustum::Frustum, mat4::Mat4, vec3::Vec3, Rect},
        scope_profile,
    },
    renderer::{
        error::RendererError,
        flat_shader::FlatShader,
        framework::{
            framebuffer::{CullFace, DrawParameters, FrameBufferTrait},
            gl,
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            gpu_texture::GpuTexture,
            state::{ColorMask, State, StencilFunc, StencilOp},
        },
        gbuffer::GBuffer,
        light_volume::LightVolumeRenderer,
        shadow_map_renderer::{
            PointShadowMapRenderContext, PointShadowMapRenderer, SpotShadowMapRenderer,
        },
        ssao::ScreenSpaceAmbientOcclusionRenderer,
        surface::SurfaceSharedData,
        GeometryCache, QualitySettings, RenderPassStatistics, TextureCache,
    },
    scene::{camera::Camera, light::LightKind, node::Node, Scene},
};
use std::{cell::RefCell, rc::Rc};

struct AmbientLightShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
    ambient_color: UniformLocation,
    ao_sampler: UniformLocation,
    ambient_texture: UniformLocation,
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
    flat_shader: FlatShader,
    spot_shadow_map_renderer: SpotShadowMapRenderer,
    point_shadow_map_renderer: PointShadowMapRenderer,
    light_volume: LightVolumeRenderer,
}

pub(in crate) struct DeferredRendererContext<'a> {
    pub state: &'a mut State,
    pub scene: &'a Scene,
    pub camera: &'a Camera,
    pub gbuffer: &'a mut GBuffer,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub ambient_color: Color,
    pub settings: &'a QualitySettings,
    pub textures: &'a mut TextureCache,
    pub geometry_cache: &'a mut GeometryCache,
}

impl DeferredLightRenderer {
    pub fn new(
        state: &mut State,
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
            sphere: SurfaceSharedData::make_sphere(6, 6, 1.0),
            flat_shader: FlatShader::new()?,
            spot_shadow_map_renderer: SpotShadowMapRenderer::new(
                state,
                settings.spot_shadow_map_size,
            )?,
            point_shadow_map_renderer: PointShadowMapRenderer::new(
                state,
                settings.point_shadow_map_size,
            )?,
            light_volume: LightVolumeRenderer::new()?,
        })
    }

    pub fn set_quality_settings(
        &mut self,
        state: &mut State,
        settings: &QualitySettings,
    ) -> Result<(), RendererError> {
        if settings.spot_shadow_map_size != self.spot_shadow_map_renderer.size {
            self.spot_shadow_map_renderer =
                SpotShadowMapRenderer::new(state, settings.spot_shadow_map_size)?;
        }
        if settings.point_shadow_map_size != self.point_shadow_map_renderer.size {
            self.point_shadow_map_renderer =
                PointShadowMapRenderer::new(state, settings.point_shadow_map_size)?;
        }
        self.ssao_renderer.set_radius(settings.ssao_radius);
        Ok(())
    }

    pub fn set_frame_size(
        &mut self,
        state: &mut State,
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
    pub(in crate) fn render(&mut self, args: DeferredRendererContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

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
        } = args;

        let viewport = Rect::new(0, 0, gbuffer.width, gbuffer.height);
        let frustum = Frustum::from(camera.view_projection_matrix()).unwrap();

        let frame_matrix = Mat4::ortho(0.0, viewport.w as f32, viewport.h as f32, 0.0, -1.0, 1.0)
            * Mat4::scale(Vec3::new(viewport.w as f32, viewport.h as f32, 0.0));

        let projection_matrix = camera.projection_matrix();
        let view_projection = camera.view_projection_matrix();
        let inv_view_projection = view_projection.inverse().unwrap_or_default();

        // Fill SSAO map.
        if settings.use_ssao {
            statistics += self.ssao_renderer.render(
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

        // Ambient light.
        gbuffer.final_frame.draw(
            geometry_cache.get(state, &self.quad),
            state,
            viewport,
            &self.ambient_light_shader.program,
            DrawParameters {
                cull_face: CullFace::Back,
                culling: false,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: false,
                depth_test: false,
                blend: false,
            },
            &[
                (
                    self.ambient_light_shader.wvp_matrix,
                    UniformValue::Mat4(frame_matrix),
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

        state.set_blend(true);
        state.set_blend_func(gl::ONE, gl::ONE);

        for light in scene.graph.linear_iter().filter_map(|node| {
            if let Node::Light(light) = node {
                Some(light)
            } else {
                None
            }
        }) {
            if !light.global_visibility() {
                continue;
            }

            let raw_radius = match light.kind() {
                LightKind::Spot(spot_light) => spot_light.distance(),
                LightKind::Point(point_light) => point_light.radius(),
                LightKind::Directional => std::f32::MAX,
            };

            let light_position = light.global_position();
            let light_radius_scale = light.local_transform().scale().max_value();
            let light_radius = light_radius_scale * raw_radius;
            let light_r_inflate = 1.05 * light_radius;
            let light_radius_vec = Vec3::new(light_r_inflate, light_r_inflate, light_r_inflate);
            let emit_direction = light.up_vector().normalized().unwrap_or(Vec3::LOOK);

            if !frustum.is_intersects_sphere(light_position, light_radius) {
                continue;
            }

            let distance_to_camera = (light.global_position() - camera.global_position()).len();

            let mut light_view_projection = Mat4::IDENTITY;
            let shadows_enabled = light.is_cast_shadows()
                && match light.kind() {
                    LightKind::Spot(spot)
                        if distance_to_camera <= settings.spot_shadows_distance
                            && settings.spot_shadows_enabled =>
                    {
                        let light_projection_matrix =
                            Mat4::perspective(spot.full_cone_angle(), 1.0, 0.01, light_radius);

                        let light_look_at = light_position - emit_direction;

                        let light_up_vec = light.look_vector().normalized().unwrap_or(Vec3::UP);

                        let light_view_matrix =
                            Mat4::look_at(light_position, light_look_at, light_up_vec)
                                .unwrap_or_default();

                        light_view_projection = light_projection_matrix * light_view_matrix;

                        statistics += self.spot_shadow_map_renderer.render(
                            state,
                            &scene.graph,
                            &light_view_projection,
                            white_dummy.clone(),
                            textures,
                            geometry_cache,
                        );

                        true
                    }
                    LightKind::Point(_)
                        if distance_to_camera <= settings.point_shadows_distance
                            && settings.point_shadows_enabled =>
                    {
                        statistics +=
                            self.point_shadow_map_renderer
                                .render(PointShadowMapRenderContext {
                                    state,
                                    graph: &scene.graph,
                                    white_dummy: white_dummy.clone(),
                                    light_pos: light_position,
                                    light_radius,
                                    texture_cache: textures,
                                    geom_cache: geometry_cache,
                                });

                        true
                    }
                    LightKind::Directional => {
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

            statistics += gbuffer.final_frame.draw(
                sphere,
                state,
                viewport,
                &self.flat_shader.program,
                DrawParameters {
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
                    UniformValue::Mat4(
                        view_projection
                            * Mat4::translate(light_position)
                            * Mat4::scale(light_radius_vec),
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

            statistics += gbuffer.final_frame.draw(
                sphere,
                state,
                viewport,
                &self.flat_shader.program,
                DrawParameters {
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
                    UniformValue::Mat4(
                        view_projection
                            * Mat4::translate(light_position)
                            * Mat4::scale(light_radius_vec),
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

            statistics += match light.kind() {
                LightKind::Spot(spot_light) => {
                    let shader = &self.spot_light_shader;

                    let uniforms = [
                        (shader.shadows_enabled, UniformValue::Bool(shadows_enabled)),
                        (
                            shader.light_view_proj_matrix,
                            UniformValue::Mat4(light_view_projection),
                        ),
                        (
                            shader.soft_shadows,
                            UniformValue::Bool(settings.spot_soft_shadows),
                        ),
                        (shader.light_position, UniformValue::Vec3(light_position)),
                        (shader.light_direction, UniformValue::Vec3(emit_direction)),
                        (shader.light_radius, UniformValue::Float(light_radius)),
                        (
                            shader.inv_view_proj_matrix,
                            UniformValue::Mat4(inv_view_projection),
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
                        (shader.wvp_matrix, UniformValue::Mat4(frame_matrix)),
                        (
                            shader.shadow_map_inv_size,
                            UniformValue::Float(1.0 / (self.spot_shadow_map_renderer.size as f32)),
                        ),
                        (
                            shader.camera_position,
                            UniformValue::Vec3(camera.global_position()),
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
                                texture: self.spot_shadow_map_renderer.texture(),
                            },
                        ),
                    ];

                    gbuffer.final_frame.draw(
                        quad,
                        state,
                        viewport,
                        &shader.program,
                        draw_params,
                        &uniforms,
                    )
                }
                LightKind::Point(_) => {
                    let shader = &self.point_light_shader;

                    let uniforms = [
                        (shader.shadows_enabled, UniformValue::Bool(shadows_enabled)),
                        (
                            shader.soft_shadows,
                            UniformValue::Bool(settings.point_soft_shadows),
                        ),
                        (shader.light_position, UniformValue::Vec3(light_position)),
                        (shader.light_radius, UniformValue::Float(light_radius)),
                        (
                            shader.inv_view_proj_matrix,
                            UniformValue::Mat4(inv_view_projection),
                        ),
                        (shader.light_color, UniformValue::Color(light.color())),
                        (shader.wvp_matrix, UniformValue::Mat4(frame_matrix)),
                        (
                            shader.camera_position,
                            UniformValue::Vec3(camera.global_position()),
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
                                texture: self.point_shadow_map_renderer.texture(),
                            },
                        ),
                    ];

                    gbuffer.final_frame.draw(
                        quad,
                        state,
                        viewport,
                        &shader.program,
                        draw_params,
                        &uniforms,
                    )
                }
                LightKind::Directional => {
                    let shader = &self.directional_light_shader;

                    let uniforms = [
                        (shader.light_direction, UniformValue::Vec3(emit_direction)),
                        (
                            shader.inv_view_proj_matrix,
                            UniformValue::Mat4(inv_view_projection),
                        ),
                        (shader.light_color, UniformValue::Color(light.color())),
                        (shader.wvp_matrix, UniformValue::Mat4(frame_matrix)),
                        (
                            shader.camera_position,
                            UniformValue::Vec3(camera.global_position()),
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

                    gbuffer.final_frame.draw(
                        quad,
                        state,
                        viewport,
                        &shader.program,
                        DrawParameters {
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
                statistics += self.light_volume.render_volume(
                    state,
                    light,
                    gbuffer,
                    &self.quad,
                    geometry_cache,
                    camera.view_matrix(),
                    projection_matrix.inverse().unwrap_or_default(),
                    camera.view_projection_matrix(),
                    viewport,
                );
            }
        }

        statistics
    }
}
