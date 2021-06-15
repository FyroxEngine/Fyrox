use crate::renderer::skybox_shader::SkyboxShader;
use crate::scene::mesh::buffer::GeometryBuffer;
use crate::scene::mesh::buffer::VertexBuffer;
use crate::scene::mesh::vertex::SimpleVertex;
use crate::{
    core::{
        algebra::{Matrix4, Point3, Vector2, Vector3},
        color::Color,
        math::{frustum::Frustum, Matrix4Ext, Rect, TriangleDefinition},
        scope_profile,
    },
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{CullFace, DrawParameters},
        gpu_program::{GpuProgram, UniformLocation},
        gpu_texture::GpuTexture,
        state::{ColorMask, PipelineState, StencilFunc, StencilOp},
    },
    renderer::{
        batch::BatchStorage,
        flat_shader::FlatShader,
        gbuffer::GBuffer,
        light_volume::LightVolumeRenderer,
        shadow_map_renderer::{
            PointShadowMapRenderContext, PointShadowMapRenderer, SpotShadowMapRenderer,
        },
        ssao::ScreenSpaceAmbientOcclusionRenderer,
        GeometryCache, QualitySettings, RenderPassStatistics, TextureCache,
    },
    scene::{camera::Camera, light::Light, mesh::surface::SurfaceData, node::Node, Scene},
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
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/ambient_light_fs.glsl");
        let vertex_source = include_str!("shaders/ambient_light_vs.glsl");
        let program =
            GpuProgram::from_source(state, "AmbientLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            ambient_color: program.uniform_location(state, "ambientColor")?,
            ao_sampler: program.uniform_location(state, "aoSampler")?,
            ambient_texture: program.uniform_location(state, "ambientTexture")?,
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
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/deferred_spot_light_fs.glsl");
        let vertex_source = include_str!("shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source(state, "DeferredLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            depth_sampler: program.uniform_location(state, "depthTexture")?,
            color_sampler: program.uniform_location(state, "colorTexture")?,
            normal_sampler: program.uniform_location(state, "normalTexture")?,
            spot_shadow_texture: program.uniform_location(state, "spotShadowTexture")?,
            cookie_enabled: program.uniform_location(state, "cookieEnabled")?,
            cookie_texture: program.uniform_location(state, "cookieTexture")?,
            light_view_proj_matrix: program.uniform_location(state, "lightViewProjMatrix")?,
            shadows_enabled: program.uniform_location(state, "shadowsEnabled")?,
            soft_shadows: program.uniform_location(state, "softShadows")?,
            shadow_map_inv_size: program.uniform_location(state, "shadowMapInvSize")?,
            light_position: program.uniform_location(state, "lightPos")?,
            light_radius: program.uniform_location(state, "lightRadius")?,
            light_color: program.uniform_location(state, "lightColor")?,
            light_direction: program.uniform_location(state, "lightDirection")?,
            half_hotspot_cone_angle_cos: program
                .uniform_location(state, "halfHotspotConeAngleCos")?,
            half_cone_angle_cos: program.uniform_location(state, "halfConeAngleCos")?,
            inv_view_proj_matrix: program.uniform_location(state, "invViewProj")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
            shadow_bias: program.uniform_location(state, "shadowBias")?,

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
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/deferred_point_light_fs.glsl");
        let vertex_source = include_str!("shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source(state, "DeferredLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            depth_sampler: program.uniform_location(state, "depthTexture")?,
            color_sampler: program.uniform_location(state, "colorTexture")?,
            normal_sampler: program.uniform_location(state, "normalTexture")?,
            point_shadow_texture: program.uniform_location(state, "pointShadowTexture")?,
            shadows_enabled: program.uniform_location(state, "shadowsEnabled")?,
            soft_shadows: program.uniform_location(state, "softShadows")?,
            light_position: program.uniform_location(state, "lightPos")?,
            light_radius: program.uniform_location(state, "lightRadius")?,
            light_color: program.uniform_location(state, "lightColor")?,
            inv_view_proj_matrix: program.uniform_location(state, "invViewProj")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
            shadow_bias: program.uniform_location(state, "shadowBias")?,

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
    fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/deferred_directional_light_fs.glsl");
        let vertex_source = include_str!("shaders/deferred_light_vs.glsl");
        let program =
            GpuProgram::from_source(state, "DeferredLightShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            depth_sampler: program.uniform_location(state, "depthTexture")?,
            color_sampler: program.uniform_location(state, "colorTexture")?,
            normal_sampler: program.uniform_location(state, "normalTexture")?,
            light_direction: program.uniform_location(state, "lightDirection")?,
            light_color: program.uniform_location(state, "lightColor")?,
            inv_view_proj_matrix: program.uniform_location(state, "invViewProj")?,
            camera_position: program.uniform_location(state, "cameraPosition")?,
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
    quad: SurfaceData,
    sphere: SurfaceData,
    skybox: SurfaceData,
    flat_shader: FlatShader,
    skybox_shader: SkyboxShader,
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
    ) -> Result<Self, FrameworkError> {
        let vertices = vec![
            // Front
            SimpleVertex {
                position: Vector3::new(-0.5, 0.5, -0.5),
                tex_coord: Vector2::new(0.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, 0.5, -0.5),
                tex_coord: Vector2::new(1.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, -0.5, -0.5),
                tex_coord: Vector2::new(1.0, 1.0),
            },
            SimpleVertex {
                position: Vector3::new(-0.5, -0.5, -0.5),
                tex_coord: Vector2::new(0.0, 1.0),
            },
            // Back
            SimpleVertex {
                position: Vector3::new(0.5, 0.5, 0.5),
                tex_coord: Vector2::new(0.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(-0.5, 0.5, 0.5),
                tex_coord: Vector2::new(1.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(-0.5, -0.5, 0.5),
                tex_coord: Vector2::new(1.0, 1.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, -0.5, 0.5),
                tex_coord: Vector2::new(0.0, 1.0),
            },
            // Left
            SimpleVertex {
                position: Vector3::new(0.5, 0.5, -0.5),
                tex_coord: Vector2::new(0.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, 0.5, 0.5),
                tex_coord: Vector2::new(1.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, -0.5, 0.5),
                tex_coord: Vector2::new(1.0, 1.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, -0.5, -0.5),
                tex_coord: Vector2::new(0.0, 1.0),
            },
            // Right
            SimpleVertex {
                position: Vector3::new(-0.5, 0.5, 0.5),
                tex_coord: Vector2::new(0.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(-0.5, 0.5, -0.5),
                tex_coord: Vector2::new(1.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(-0.5, -0.5, -0.5),
                tex_coord: Vector2::new(1.0, 1.0),
            },
            SimpleVertex {
                position: Vector3::new(-0.5, -0.5, 0.5),
                tex_coord: Vector2::new(0.0, 1.0),
            },
            // Up
            SimpleVertex {
                position: Vector3::new(-0.5, 0.5, 0.5),
                tex_coord: Vector2::new(0.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, 0.5, 0.5),
                tex_coord: Vector2::new(1.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, 0.5, -0.5),
                tex_coord: Vector2::new(1.0, 1.0),
            },
            SimpleVertex {
                position: Vector3::new(-0.5, 0.5, -0.5),
                tex_coord: Vector2::new(0.0, 1.0),
            },
            // Down
            SimpleVertex {
                position: Vector3::new(-0.5, -0.5, 0.5),
                tex_coord: Vector2::new(0.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, -0.5, 0.5),
                tex_coord: Vector2::new(1.0, 0.0),
            },
            SimpleVertex {
                position: Vector3::new(0.5, -0.5, -0.5),
                tex_coord: Vector2::new(1.0, 1.0),
            },
            SimpleVertex {
                position: Vector3::new(-0.5, -0.5, -0.5),
                tex_coord: Vector2::new(0.0, 1.0),
            },
        ];

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
            quad: SurfaceData::make_unit_xy_quad(),
            skybox: SurfaceData::new(
                VertexBuffer::new(vertices.len(), SimpleVertex::layout(), vertices).unwrap(),
                GeometryBuffer::new(vec![
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
                true,
            ),
            sphere: SurfaceData::make_sphere(6, 6, 1.0, &Matrix4::identity()),
            flat_shader: FlatShader::new(state)?,
            skybox_shader: SkyboxShader::new(state)?,
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
            light_volume: LightVolumeRenderer::new(state)?,
        })
    }

    pub fn set_quality_settings(
        &mut self,
        state: &mut PipelineState,
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
        self.ssao_renderer.set_radius(settings.ssao_radius);
        Ok(())
    }

    pub fn set_frame_size(
        &mut self,
        state: &mut PipelineState,
        frame_size: (u32, u32),
    ) -> Result<(), FrameworkError> {
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
        let frustum = Frustum::from(camera.view_projection_matrix()).unwrap_or_default();

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

            if let Some(gpu_texture) = textures.get(state, &skybox.cubemap().clone().unwrap()) {
                let shader = &self.skybox_shader;
                pass_stats += gbuffer
                    .final_frame
                    .draw_part(
                        geometry_cache.get(state, &self.skybox),
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
                            blend: false,
                        },
                        0,
                        12,
                        |program_binding| {
                            program_binding
                                .set_texture(&shader.cubemap_texture, &gpu_texture)
                                .set_matrix4(&shader.wvp_matrix, &(view_projection * wvp));
                        },
                    )
                    .unwrap();
            }
        }

        state.set_blend(true);
        state.set_blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

        // Ambient light.
        let gbuffer_depth_map = gbuffer.depth();
        let gbuffer_diffuse_map = gbuffer.diffuse_texture();
        let gbuffer_normal_map = gbuffer.normal_texture();
        let gbuffer_ambient_map = gbuffer.ambient_texture();
        let ao_map = self.ssao_renderer.ao_map();

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
            |program_binding| {
                program_binding
                    .set_matrix4(&self.ambient_light_shader.wvp_matrix, &frame_matrix)
                    .set_color(&self.ambient_light_shader.ambient_color, &ambient_color)
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
        );

        state.set_blend_func(glow::ONE, glow::ONE);

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
            let cascade_index =
                if distance_to_camera < b1 || frustum.is_contains_point(camera.global_position()) {
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
                func: glow::ALWAYS,
                ..Default::default()
            });
            state.set_stencil_op(StencilOp {
                zfail: glow::INCR,
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
                |program_binding| {
                    program_binding.set_matrix4(
                        &self.flat_shader.wvp_matrix,
                        &(view_projection
                            * Matrix4::new_translation(&light_position)
                            * Matrix4::new_nonuniform_scaling(&light_radius_vec)),
                    );
                },
            );

            state.set_stencil_func(StencilFunc {
                func: glow::ALWAYS,
                ..Default::default()
            });
            state.set_stencil_op(StencilOp {
                zfail: glow::DECR,
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
                |program_binding| {
                    program_binding.set_matrix4(
                        &self.flat_shader.wvp_matrix,
                        &(view_projection
                            * Matrix4::new_translation(&light_position)
                            * Matrix4::new_nonuniform_scaling(&light_radius_vec)),
                    );
                },
            );

            state.set_stencil_func(StencilFunc {
                func: glow::NOTEQUAL,
                ..Default::default()
            });
            state.set_stencil_op(StencilOp {
                zpass: glow::ZERO,
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
                            if let Some(cookie) = textures.get(state, texture) {
                                (true, cookie)
                            } else {
                                (false, white_dummy.clone())
                            }
                        } else {
                            (false, white_dummy.clone())
                        };

                    light_stats.spot_lights_rendered += 1;

                    gbuffer.final_frame.draw(
                        quad,
                        state,
                        viewport,
                        &shader.program,
                        &draw_params,
                        |program_binding| {
                            program_binding
                                .set_bool(&shader.shadows_enabled, shadows_enabled)
                                .set_matrix4(&shader.light_view_proj_matrix, &light_view_projection)
                                .set_bool(&shader.soft_shadows, settings.spot_soft_shadows)
                                .set_vector3(&shader.light_position, &light_position)
                                .set_vector3(&shader.light_direction, &emit_direction)
                                .set_float(&shader.light_radius, light_radius)
                                .set_matrix4(&shader.inv_view_proj_matrix, &inv_view_projection)
                                .set_color(&shader.light_color, &light.color())
                                .set_float(
                                    &shader.half_hotspot_cone_angle_cos,
                                    (spot_light.hotspot_cone_angle() * 0.5).cos(),
                                )
                                .set_float(
                                    &shader.half_cone_angle_cos,
                                    (spot_light.full_cone_angle() * 0.5).cos(),
                                )
                                .set_matrix4(&shader.wvp_matrix, &frame_matrix)
                                .set_float(
                                    &shader.shadow_map_inv_size,
                                    1.0 / (self.spot_shadow_map_renderer.cascade_size(cascade_index)
                                        as f32),
                                )
                                .set_vector3(&shader.camera_position, &camera_global_position)
                                .set_texture(&shader.depth_sampler, &gbuffer_depth_map)
                                .set_texture(&shader.color_sampler, &gbuffer_diffuse_map)
                                .set_texture(&shader.normal_sampler, &gbuffer_normal_map)
                                .set_texture(
                                    &shader.spot_shadow_texture,
                                    &self.spot_shadow_map_renderer.cascade_texture(cascade_index),
                                )
                                .set_texture(&shader.cookie_texture, &cookie_texture)
                                .set_bool(&shader.cookie_enabled, cookie_enabled)
                                .set_float(&shader.shadow_bias, spot_light.shadow_bias());
                        },
                    )
                }
                Light::Point(point_light) => {
                    let shader = &self.point_light_shader;

                    light_stats.point_lights_rendered += 1;

                    gbuffer.final_frame.draw(
                        quad,
                        state,
                        viewport,
                        &shader.program,
                        &draw_params,
                        |program_binding| {
                            program_binding
                                .set_bool(&shader.shadows_enabled, shadows_enabled)
                                .set_bool(&shader.soft_shadows, settings.point_soft_shadows)
                                .set_vector3(&shader.light_position, &light_position)
                                .set_float(&shader.light_radius, light_radius)
                                .set_matrix4(&shader.inv_view_proj_matrix, &inv_view_projection)
                                .set_color(&shader.light_color, &light.color())
                                .set_matrix4(&shader.wvp_matrix, &frame_matrix)
                                .set_vector3(&shader.camera_position, &camera_global_position)
                                .set_float(&shader.shadow_bias, point_light.shadow_bias())
                                .set_texture(&shader.depth_sampler, &gbuffer_depth_map)
                                .set_texture(&shader.color_sampler, &gbuffer_diffuse_map)
                                .set_texture(&shader.normal_sampler, &gbuffer_normal_map)
                                .set_texture(
                                    &shader.point_shadow_texture,
                                    &self
                                        .point_shadow_map_renderer
                                        .cascade_texture(cascade_index),
                                );
                        },
                    )
                }
                Light::Directional(_) => {
                    let shader = &self.directional_light_shader;

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
                        |program_binding| {
                            program_binding
                                .set_vector3(&shader.light_direction, &emit_direction)
                                .set_matrix4(&shader.inv_view_proj_matrix, &inv_view_projection)
                                .set_color(&shader.light_color, &light.color())
                                .set_matrix4(&shader.wvp_matrix, &frame_matrix)
                                .set_vector3(&shader.camera_position, &camera_global_position)
                                .set_texture(&shader.depth_sampler, &gbuffer_depth_map)
                                .set_texture(&shader.color_sampler, &gbuffer_diffuse_map)
                                .set_texture(&shader.normal_sampler, &gbuffer_normal_map);
                        },
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
                    inv_projection,
                    view_projection,
                    viewport,
                    &scene.graph,
                );
            }
        }

        (pass_stats, light_stats)
    }
}
