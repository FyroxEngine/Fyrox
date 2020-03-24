use std::{
    rc::Rc,
    cell::RefCell,
};
use crate::{
    renderer::{
        flat_shader::FlatShader,
        surface::SurfaceSharedData,
        framework::{
            gl,
            gpu_texture::GpuTexture,
            gpu_program::{
                UniformLocation,
                GpuProgram,
                UniformValue,
            },
            framebuffer::{
                DrawParameters,
                CullFace,
                FrameBufferTrait,
            },
            state::{
                State,
                ColorMask,
                StencilFunc,
                StencilOp
            }
        },
        gbuffer::GBuffer,
        error::RendererError,
        shadow_map_renderer::{
            SpotShadowMapRenderer,
            PointShadowMapRenderer,
        },
        QualitySettings,
        RenderPassStatistics,
        GeometryCache,
        TextureCache,
        ssao::ScreenSpaceAmbientOcclusionRenderer
    },
    scene::{
        camera::Camera,
        Scene,
        node::Node,
        light::LightKind,
        base::AsBase,
    },
    core::{
        math::{
            vec3::Vec3,
            mat4::Mat4,
            frustum::Frustum,
            Rect,
        },
        color::Color,
    },
};

struct AmbientLightShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    diffuse_texture: UniformLocation,
    ambient_color: UniformLocation,
    ao_sampler: UniformLocation
}

impl AmbientLightShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/ambient_light_fs.glsl");
        let vertex_source = include_str!("shaders/ambient_light_vs.glsl");
        let program = GpuProgram::from_source("AmbientLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            ambient_color: program.uniform_location("ambientColor")?,
            ao_sampler: program.uniform_location("aoSampler")?,
            program,
        })
    }
}

struct DeferredLightingShader {
    program: GpuProgram,
    wvp_matrix: UniformLocation,
    depth_sampler: UniformLocation,
    color_sampler: UniformLocation,
    normal_sampler: UniformLocation,
    spot_shadow_texture: UniformLocation,
    point_shadow_texture: UniformLocation,
    light_view_proj_matrix: UniformLocation,
    light_type: UniformLocation,
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

impl DeferredLightingShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/deferred_light_fs.glsl");
        let vertex_source = include_str!("shaders/deferred_light_vs.glsl");
        let program = GpuProgram::from_source("DeferredLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            depth_sampler: program.uniform_location("depthTexture")?,
            color_sampler: program.uniform_location("colorTexture")?,
            normal_sampler: program.uniform_location("normalTexture")?,
            spot_shadow_texture: program.uniform_location("spotShadowTexture")?,
            point_shadow_texture: program.uniform_location("pointShadowTexture")?,
            light_view_proj_matrix: program.uniform_location("lightViewProjMatrix")?,
            light_type: program.uniform_location("lightType")?,
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

pub struct DeferredLightRenderer {
    pub ssao_renderer: ScreenSpaceAmbientOcclusionRenderer,
    shader: DeferredLightingShader,
    ambient_light_shader: AmbientLightShader,
    quad: SurfaceSharedData,
    sphere: SurfaceSharedData,
    flat_shader: FlatShader,
    spot_shadow_map_renderer: SpotShadowMapRenderer,
    point_shadow_map_renderer: PointShadowMapRenderer,
}

pub struct DeferredRendererContext<'a> {
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
    pub fn new(state: &mut State, frame_size: (u32, u32), settings: &QualitySettings) -> Result<Self, RendererError> {
        Ok(Self {
            ssao_renderer: ScreenSpaceAmbientOcclusionRenderer::new(state, frame_size.0 as usize, frame_size.1 as usize)?,
            shader: DeferredLightingShader::new()?,
            ambient_light_shader: AmbientLightShader::new()?,
            quad: SurfaceSharedData::make_unit_xy_quad(),
            sphere: SurfaceSharedData::make_sphere(6, 6, 1.0),
            flat_shader: FlatShader::new()?,
            spot_shadow_map_renderer: SpotShadowMapRenderer::new(state, settings.spot_shadow_map_size)?,
            point_shadow_map_renderer: PointShadowMapRenderer::new(state, settings.point_shadow_map_size)?,
        })
    }

    pub fn set_quality_settings(&mut self, state: &mut State, settings: &QualitySettings) -> Result<(), RendererError> {
        if settings.spot_shadow_map_size != self.spot_shadow_map_renderer.size {
            self.spot_shadow_map_renderer = SpotShadowMapRenderer::new(state, settings.spot_shadow_map_size)?;
        }
        if settings.point_shadow_map_size != self.point_shadow_map_renderer.size {
            self.point_shadow_map_renderer = PointShadowMapRenderer::new(state, settings.point_shadow_map_size)?;
        }
        self.ssao_renderer.set_radius(settings.ssao_radius);
        Ok(())
    }

    pub fn set_frame_size(&mut self, state: &mut State, frame_size: (u32, u32)) -> Result<(), RendererError>{
        self.ssao_renderer = ScreenSpaceAmbientOcclusionRenderer::new(state, frame_size.0 as usize, frame_size.1 as usize)?;
        Ok(())
    }

    #[must_use]
    pub fn render(&mut self, context: DeferredRendererContext) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        let viewport = Rect::new(0, 0, context.gbuffer.width, context.gbuffer.height);
        let frustum = Frustum::from(context.camera.view_projection_matrix()).unwrap();

        let frame_matrix =
            Mat4::ortho(0.0, viewport.w as f32, viewport.h as f32, 0.0, -1.0, 1.0) *
                Mat4::scale(Vec3::new(viewport.w as f32, viewport.h as f32, 0.0));

        let projection_matrix = context.camera.projection_matrix();
        let view_projection = context.camera.view_projection_matrix();
        let inv_view_projection = view_projection.inverse().unwrap_or_default();

        // Fill SSAO map.
        if context.settings.use_ssao {
            statistics += self.ssao_renderer.render(
                context.state,
                context.gbuffer,
                context.geometry_cache,
                projection_matrix,
                context.camera.view_matrix().basis()
            );
        }

        context.gbuffer.opt_framebuffer.clear(context.state, viewport, Some(Color::from_rgba(0, 0, 0, 0)), None, Some(0));

        // Ambient light.
        context.gbuffer.opt_framebuffer.draw(
            context.state,
            viewport,
            context.geometry_cache.get(&self.quad),
            &mut self.ambient_light_shader.program,
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
                (self.ambient_light_shader.wvp_matrix, UniformValue::Mat4(frame_matrix)),
                (self.ambient_light_shader.ambient_color, UniformValue::Color(context.ambient_color)),
                (self.ambient_light_shader.diffuse_texture, UniformValue::Sampler {
                    index: 0,
                    texture: context.gbuffer.diffuse_texture(),
                }),
                (self.ambient_light_shader.ao_sampler, UniformValue::Sampler {
                    index: 1,
                    texture: if context.settings.use_ssao {
                        self.ssao_renderer.ao_map()
                    } else {
                        context.white_dummy.clone()
                    }
                })
            ],
        );

        context.state.set_blend(true);
        context.state.set_blend_func(gl::ONE, gl::ONE);

        for light in context.scene.graph.linear_iter().filter_map(|node| {
            if let Node::Light(light) = node { Some(light) } else { None }
        }) {
            if !light.base().global_visibility() {
                continue;
            }

            let raw_radius = match light.get_kind() {
                LightKind::Spot(spot_light) => spot_light.distance(),
                LightKind::Point(point_light) => point_light.get_radius(),
            };

            let light_position = light.base().global_position();
            let light_radius_scale = light.base().local_transform().scale().max_value();
            let light_radius = light_radius_scale * raw_radius;
            let light_r_inflate = 1.05 * light_radius;
            let light_radius_vec = Vec3::new(light_r_inflate, light_r_inflate, light_r_inflate);
            let emit_direction = light.base().up_vector().normalized().unwrap_or(Vec3::LOOK);

            if !frustum.is_intersects_sphere(light_position, light_radius) {
                continue;
            }

            let distance_to_camera = (light.base().global_position() - context.camera.base().global_position()).len();

            let mut light_view_projection = Mat4::IDENTITY;
            let apply_shadows = match light.get_kind() {
                LightKind::Spot(spot) if distance_to_camera <= context.settings.spot_shadows_distance && context.settings.spot_shadows_enabled => {
                    let light_projection_matrix = Mat4::perspective(
                        spot.full_cone_angle(),
                        1.0,
                        0.01,
                        light_radius,
                    );

                    let light_look_at = light_position - emit_direction;

                    let light_up_vec = light.base().look_vector().normalized().unwrap_or(Vec3::UP);

                    let light_view_matrix = Mat4::look_at(light_position, light_look_at, light_up_vec)
                        .unwrap_or_default();

                    light_view_projection = light_projection_matrix * light_view_matrix;

                    statistics += self.spot_shadow_map_renderer.render(
                        context.state,
                        &context.scene.graph,
                        &light_view_projection,
                        context.white_dummy.clone(),
                        context.textures,
                        context.geometry_cache,
                    );

                    true
                }
                LightKind::Point(_) if distance_to_camera <= context.settings.point_shadows_distance && context.settings.point_shadows_enabled => {
                    statistics += self.point_shadow_map_renderer.render(
                        context.state,
                        &context.scene.graph,
                        context.white_dummy.clone(),
                        light_position,
                        light_radius,
                        context.textures,
                        context.geometry_cache,
                    );

                    true
                }
                _ => false
            };

            // Mark lighted areas in stencil buffer to do light calculations only on them.
            context.state.set_stencil_mask(0xFFFF_FFFF);
            context.state.set_stencil_func(StencilFunc { func: gl::ALWAYS, ..Default::default() });
            context.state.set_stencil_op(StencilOp { zfail: gl::INCR, ..Default::default() });

            statistics.add_draw_call(context.gbuffer.opt_framebuffer.draw(
                context.state,
                viewport,
                context.geometry_cache.get(&self.sphere),
                &mut self.flat_shader.program,
                DrawParameters {
                    cull_face: CullFace::Front,
                    culling: true,
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: true,
                    depth_test: true,
                    blend: false,
                },
                &[
                    (self.flat_shader.wvp_matrix, UniformValue::Mat4(
                        view_projection * Mat4::translate(light_position) * Mat4::scale(light_radius_vec)
                    ))
                ],
            ));

            context.state.set_stencil_func(StencilFunc { func: gl::ALWAYS, ..Default::default() });
            context.state.set_stencil_op(StencilOp { zfail: gl::DECR, ..Default::default() });

            statistics.add_draw_call(context.gbuffer.opt_framebuffer.draw(
                context.state,
                viewport,
                context.geometry_cache.get(&self.sphere),
                &mut self.flat_shader.program,
                DrawParameters {
                    cull_face: CullFace::Back,
                    culling: true,
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: true,
                    depth_test: true,
                    blend: false,
                },
                &[
                    (self.flat_shader.wvp_matrix, UniformValue::Mat4(
                        view_projection * Mat4::translate(light_position) * Mat4::scale(light_radius_vec)
                    ))
                ],
            ));

            context.state.set_stencil_func(StencilFunc { func: gl::NOTEQUAL, ..Default::default() });
            context.state.set_stencil_op(StencilOp { zpass: gl::ZERO, ..Default::default() });

            let (hotspot_cone_angle, cone_angle) = match light.get_kind() {
                LightKind::Spot(spot_light) => (spot_light.hotspot_cone_angle(), spot_light.full_cone_angle()),
                LightKind::Point(_) => (2.0 * std::f32::consts::PI, 2.0 * std::f32::consts::PI),
            };

            // Finally render light.
            statistics.add_draw_call(context.gbuffer.opt_framebuffer.draw(
                context.state,
                viewport,
                context.geometry_cache.get(&self.quad),
                &mut self.shader.program,
                DrawParameters {
                    cull_face: CullFace::Back,
                    culling: false,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: true,
                    depth_test: false,
                    blend: true,
                },
                &[
                    (self.shader.light_type, UniformValue::Integer({
                        match light.get_kind() {
                            LightKind::Spot(_) if apply_shadows => 2,
                            LightKind::Point(_) if apply_shadows => 0,
                            _ => -1
                        }
                    })),
                    (self.shader.light_view_proj_matrix, UniformValue::Mat4(light_view_projection)),
                    (self.shader.soft_shadows, UniformValue::Bool({
                        match light.get_kind() {
                            LightKind::Spot(_) if context.settings.spot_soft_shadows => true,
                            LightKind::Point(_) if context.settings.point_soft_shadows => true,
                            _ => false
                        }
                    })),
                    (self.shader.light_position, UniformValue::Vec3(light_position)),
                    (self.shader.light_direction, UniformValue::Vec3(emit_direction)),
                    (self.shader.light_radius, UniformValue::Float(light_radius)),
                    (self.shader.inv_view_proj_matrix, UniformValue::Mat4(inv_view_projection)),
                    (self.shader.light_color, UniformValue::Color(light.get_color())),
                    (self.shader.half_hotspot_cone_angle_cos, UniformValue::Float((hotspot_cone_angle * 0.5).cos())),
                    (self.shader.half_cone_angle_cos, UniformValue::Float((cone_angle * 0.5).cos())),
                    (self.shader.wvp_matrix, UniformValue::Mat4(frame_matrix)),
                    (self.shader.shadow_map_inv_size, UniformValue::Float(1.0 / (self.spot_shadow_map_renderer.size as f32))),
                    (self.shader.camera_position, UniformValue::Vec3(context.camera.base().global_position())),
                    (self.shader.depth_sampler, UniformValue::Sampler { index: 0, texture: context.gbuffer.depth() }),
                    (self.shader.color_sampler, UniformValue::Sampler { index: 1, texture: context.gbuffer.diffuse_texture() }),
                    (self.shader.normal_sampler, UniformValue::Sampler { index: 2, texture: context.gbuffer.normal_texture() }),
                    (self.shader.spot_shadow_texture, UniformValue::Sampler { index: 3, texture: self.spot_shadow_map_renderer.texture() }),
                    (self.shader.point_shadow_texture, UniformValue::Sampler { index: 4, texture: self.point_shadow_map_renderer.texture() })
                ]));
            check_gl_error!();
        }

        statistics
    }
}