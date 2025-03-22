// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::renderer::DynamicSurfaceCache;
use crate::{
    core::{
        algebra::{Matrix4, Point3, UnitQuaternion, Vector2, Vector3},
        color::Color,
        math::{frustum::Frustum, Matrix4Ext, Rect, TriangleDefinition},
        ImmutableString,
    },
    renderer::{
        bundle::{LightSourceKind, RenderDataBundleStorage},
        cache::{
            shader::{
                binding, property, PropertyGroup, RenderMaterial, RenderPassContainer, ShaderCache,
            },
            uniform::{UniformBufferCache, UniformMemoryAllocator},
        },
        framework::{
            buffer::BufferUsage, error::FrameworkError, framebuffer::GpuFrameBuffer,
            geometry_buffer::GpuGeometryBuffer, server::GraphicsServer, ColorMask, CompareFunc,
            CullFace, DrawParameters, ElementRange, GeometryBufferExt, StencilAction, StencilFunc,
            StencilOp,
        },
        gbuffer::GBuffer,
        light_volume::LightVolumeRenderer,
        make_viewport_matrix,
        shadow::{
            csm::{CsmRenderContext, CsmRenderer},
            point::{PointShadowMapRenderContext, PointShadowMapRenderer},
            spot::SpotShadowMapRenderer,
        },
        ssao::ScreenSpaceAmbientOcclusionRenderer,
        visibility::ObserverVisibilityCache,
        FallbackResources, GeometryCache, LightingStatistics, QualitySettings,
        RenderPassStatistics, TextureCache,
    },
    scene::{
        camera::Camera,
        mesh::{
            buffer::{TriangleBuffer, VertexBuffer},
            surface::SurfaceData,
            vertex::SimpleVertex,
        },
        Scene,
    },
};

pub struct DeferredLightRenderer {
    pub ssao_renderer: ScreenSpaceAmbientOcclusionRenderer,
    spot_light_shader: RenderPassContainer,
    point_light_shader: RenderPassContainer,
    directional_light_shader: RenderPassContainer,
    ambient_light_shader: RenderPassContainer,
    quad: GpuGeometryBuffer,
    sphere: GpuGeometryBuffer,
    cone: GpuGeometryBuffer,
    skybox: GpuGeometryBuffer,
    skybox_shader: RenderPassContainer,
    spot_shadow_map_renderer: SpotShadowMapRenderer,
    point_shadow_map_renderer: PointShadowMapRenderer,
    csm_renderer: CsmRenderer,
    light_volume: LightVolumeRenderer,
    volume_marker: RenderPassContainer,
    pixel_counter: RenderPassContainer,
}

pub(crate) struct DeferredRendererContext<'a> {
    pub elapsed_time: f32,
    pub server: &'a dyn GraphicsServer,
    pub scene: &'a Scene,
    pub camera: &'a Camera,
    pub gbuffer: &'a mut GBuffer,
    pub ambient_color: Color,
    pub render_data_bundle: &'a RenderDataBundleStorage,
    pub settings: &'a QualitySettings,
    pub textures: &'a mut TextureCache,
    pub geometry_cache: &'a mut GeometryCache,
    pub frame_buffer: &'a GpuFrameBuffer,
    pub shader_cache: &'a mut ShaderCache,
    pub fallback_resources: &'a FallbackResources,
    pub uniform_buffer_cache: &'a mut UniformBufferCache,
    pub visibility_cache: &'a mut ObserverVisibilityCache,
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,
    pub dynamic_surface_cache: &'a mut DynamicSurfaceCache,
}

impl DeferredLightRenderer {
    pub fn new(
        server: &dyn GraphicsServer,
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
                server,
                frame_size.0 as usize,
                frame_size.1 as usize,
            )?,
            spot_light_shader: RenderPassContainer::from_str(
                server,
                include_str!("shaders/deferred_spot_light.shader"),
            )?,
            point_light_shader: RenderPassContainer::from_str(
                server,
                include_str!("shaders/deferred_point_light.shader"),
            )?,
            directional_light_shader: RenderPassContainer::from_str(
                server,
                include_str!("shaders/deferred_directional_light.shader"),
            )?,
            ambient_light_shader: RenderPassContainer::from_str(
                server,
                include_str!("shaders/ambient_light.shader"),
            )?,
            quad: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
            skybox: GpuGeometryBuffer::from_surface_data(
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
                BufferUsage::StaticDraw,
                server,
            )?,
            sphere: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_sphere(10, 10, 1.0, &Matrix4::identity()),
                BufferUsage::StaticDraw,
                server,
            )?,
            cone: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_cone(
                    16,
                    0.5,
                    1.0,
                    &Matrix4::new_translation(&Vector3::new(0.0, -1.0, 0.0)),
                ),
                BufferUsage::StaticDraw,
                server,
            )?,
            skybox_shader: RenderPassContainer::from_str(
                server,
                include_str!("shaders/skybox.shader"),
            )?,
            spot_shadow_map_renderer: SpotShadowMapRenderer::new(
                server,
                settings.spot_shadow_map_size,
                quality_defaults.spot_shadow_map_precision,
            )?,
            point_shadow_map_renderer: PointShadowMapRenderer::new(
                server,
                settings.point_shadow_map_size,
                quality_defaults.point_shadow_map_precision,
            )?,
            light_volume: LightVolumeRenderer::new(server)?,
            csm_renderer: CsmRenderer::new(
                server,
                quality_defaults.csm_settings.size,
                quality_defaults.csm_settings.precision,
            )?,
            volume_marker: RenderPassContainer::from_str(
                server,
                include_str!("shaders/volume_marker_lit.shader"),
            )?,
            pixel_counter: RenderPassContainer::from_str(
                server,
                include_str!("shaders/pixel_counter.shader"),
            )?,
        })
    }

    pub fn set_quality_settings(
        &mut self,
        server: &dyn GraphicsServer,
        settings: &QualitySettings,
    ) -> Result<(), FrameworkError> {
        if settings.spot_shadow_map_size != self.spot_shadow_map_renderer.base_size()
            || settings.spot_shadow_map_precision != self.spot_shadow_map_renderer.precision()
        {
            self.spot_shadow_map_renderer = SpotShadowMapRenderer::new(
                server,
                settings.spot_shadow_map_size,
                settings.spot_shadow_map_precision,
            )?;
        }
        if settings.point_shadow_map_size != self.point_shadow_map_renderer.base_size()
            || settings.point_shadow_map_precision != self.point_shadow_map_renderer.precision()
        {
            self.point_shadow_map_renderer = PointShadowMapRenderer::new(
                server,
                settings.point_shadow_map_size,
                settings.point_shadow_map_precision,
            )?;
        }
        if settings.csm_settings.precision != self.csm_renderer.precision()
            || settings.csm_settings.size != self.csm_renderer.size()
        {
            self.csm_renderer = CsmRenderer::new(
                server,
                settings.csm_settings.size,
                settings.csm_settings.precision,
            )?;
        }
        self.ssao_renderer.set_radius(settings.ssao_radius);
        Ok(())
    }

    pub fn set_frame_size(
        &mut self,
        server: &dyn GraphicsServer,
        frame_size: (u32, u32),
    ) -> Result<(), FrameworkError> {
        self.ssao_renderer = ScreenSpaceAmbientOcclusionRenderer::new(
            server,
            frame_size.0 as usize,
            frame_size.1 as usize,
        )?;
        Ok(())
    }

    pub(crate) fn render(
        &mut self,
        args: DeferredRendererContext,
    ) -> Result<(RenderPassStatistics, LightingStatistics), FrameworkError> {
        let mut pass_stats = RenderPassStatistics::default();
        let mut light_stats = LightingStatistics::default();

        let DeferredRendererContext {
            elapsed_time,
            server,
            scene,
            camera,
            gbuffer,
            render_data_bundle,
            shader_cache,
            ambient_color,
            settings,
            textures,
            geometry_cache,
            frame_buffer,
            fallback_resources,
            uniform_buffer_cache,
            visibility_cache,
            uniform_memory_allocator,
            dynamic_surface_cache,
        } = args;

        let viewport = Rect::new(0, 0, gbuffer.width, gbuffer.height);
        let frustum = Frustum::from_view_projection_matrix(camera.view_projection_matrix())
            .unwrap_or_default();

        let frame_matrix = make_viewport_matrix(viewport);

        let projection_matrix = camera.projection_matrix();
        let view_projection = camera.view_projection_matrix();
        let inv_projection = projection_matrix.try_inverse().unwrap_or_default();
        let inv_view_projection = view_projection.try_inverse().unwrap_or_default();
        let camera_global_position = camera.global_position();

        // Fill SSAO map.
        if settings.use_ssao {
            pass_stats += self.ssao_renderer.render(
                gbuffer,
                projection_matrix,
                camera.view_matrix().basis(),
                uniform_buffer_cache,
                fallback_resources,
            )?;
        }

        // Render skybox (if any).
        if let Some(skybox) = camera.skybox_ref() {
            if let Some(texture_sampler_pair) = skybox
                .cubemap_ref()
                .and_then(|cube_map| textures.get(server, cube_map))
            {
                let size = camera.projection().z_far() / 2.0f32.sqrt();
                let scale = Matrix4::new_scaling(size);
                let wvp = Matrix4::new_translation(&camera.global_position()) * scale;
                let wvp = view_projection * wvp;
                let properties = PropertyGroup::from([property("worldViewProjection", &wvp)]);
                let material = RenderMaterial::from([
                    binding(
                        "cubemapTexture",
                        (
                            &texture_sampler_pair.gpu_texture,
                            &texture_sampler_pair.gpu_sampler,
                        ),
                    ),
                    binding("properties", &properties),
                ]);

                pass_stats += self.skybox_shader.run_pass(
                    1,
                    &ImmutableString::new("Primary"),
                    frame_buffer,
                    &self.skybox,
                    viewport,
                    &material,
                    uniform_buffer_cache,
                    ElementRange::Specific {
                        offset: 0,
                        count: 12,
                    },
                    None,
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

        let ambient_color = ambient_color.srgb_to_linear_f32();
        let properties = PropertyGroup::from([
            property("worldViewProjection", &frame_matrix),
            property("ambientColor", &ambient_color),
        ]);
        let material = RenderMaterial::from([
            binding(
                "diffuseTexture",
                (
                    gbuffer_diffuse_map,
                    &fallback_resources.nearest_clamp_sampler,
                ),
            ),
            binding(
                "aoSampler",
                if settings.use_ssao {
                    (&ao_map, &fallback_resources.linear_clamp_sampler)
                } else {
                    (
                        &fallback_resources.white_dummy,
                        &fallback_resources.linear_clamp_sampler,
                    )
                },
            ),
            binding(
                "ambientTexture",
                (
                    gbuffer_ambient_map,
                    &fallback_resources.nearest_clamp_sampler,
                ),
            ),
            binding("properties", &properties),
        ]);

        pass_stats += self.ambient_light_shader.run_pass(
            1,
            &ImmutableString::new("Primary"),
            frame_buffer,
            &self.quad,
            viewport,
            &material,
            uniform_buffer_cache,
            Default::default(),
            None,
        )?;

        for light in render_data_bundle.light_sources.iter() {
            let distance_to_camera = (light.position - camera.global_position()).norm();

            let (
                raw_radius,
                shadows_distance,
                shadows_enabled,
                shadows_fade_out_range,
                bounding_shape,
                shape_specific_matrix,
            ) = match light.kind {
                LightSourceKind::Spot {
                    full_cone_angle,
                    distance,
                    ..
                } => {
                    let margin = 2.0f32.to_radians();
                    // Angle at the top vertex of the right triangle with vertical side be 1.0 and horizontal
                    // side be 0.5.
                    let vertex_angle = 26.56f32.to_radians();
                    let k_angle = (full_cone_angle * 0.5 + margin).tan() / vertex_angle.tan();
                    (
                        distance,
                        settings.spot_shadows_distance,
                        light.cast_shadows
                            && distance_to_camera <= settings.spot_shadows_distance
                            && settings.spot_shadows_enabled,
                        settings.spot_shadows_fade_out_range,
                        &self.cone,
                        Matrix4::new_nonuniform_scaling(&Vector3::new(
                            distance * k_angle,
                            distance * 1.05,
                            distance * k_angle,
                        )),
                    )
                }
                LightSourceKind::Point { radius, .. } => (
                    radius,
                    settings.point_shadows_distance,
                    light.cast_shadows
                        && distance_to_camera <= settings.point_shadows_distance
                        && settings.point_shadows_enabled,
                    settings.point_shadows_fade_out_range,
                    &self.sphere,
                    Matrix4::new_scaling(radius * 1.05),
                ),
                LightSourceKind::Directional { .. } => {
                    (
                        f32::MAX,
                        0.0,
                        light.cast_shadows && settings.csm_settings.enabled,
                        0.0,
                        // Makes no sense, but whatever.
                        &self.sphere,
                        Matrix4::identity(),
                    )
                }
                LightSourceKind::Unknown => {
                    continue;
                }
            };

            let scl = light.local_scale;
            let light_radius_scale = scl.x.max(scl.y).max(scl.z);
            let light_radius = light_radius_scale * raw_radius;
            let light_rotation = UnitQuaternion::from_matrix_eps(
                &light.global_transform.basis(),
                10.0 * f32::EPSILON,
                16,
                Default::default(),
            )
            .to_homogeneous();
            let bounding_shape_matrix =
                Matrix4::new_translation(&light.position) * light_rotation * shape_specific_matrix;
            let emit_direction = light
                .up_vector
                .try_normalize(f32::EPSILON)
                .unwrap_or_else(Vector3::z);

            if !frustum.is_intersects_sphere(light.position, light_radius) {
                continue;
            }

            let b1 = shadows_distance * 0.2;
            let b2 = shadows_distance * 0.4;
            let cascade_index = if distance_to_camera < b1
                || (camera.global_position().metric_distance(&light.position) <= light_radius)
            {
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

            // Mark lit areas in stencil buffer to do light calculations only on them.
            let shape_wvp_matrix = view_projection * bounding_shape_matrix;
            for (cull_face, stencil_action) in [
                (CullFace::Front, StencilAction::Incr),
                (CullFace::Back, StencilAction::Decr),
            ] {
                let draw_params = DrawParameters {
                    cull_face: Some(cull_face),
                    color_write: ColorMask::all(false),
                    depth_write: false,
                    stencil_test: Some(StencilFunc {
                        func: CompareFunc::Always,
                        ..Default::default()
                    }),
                    stencil_op: StencilOp {
                        zfail: stencil_action,
                        ..Default::default()
                    },
                    depth_test: Some(CompareFunc::Less),
                    blend: None,
                    scissor_box: None,
                };
                let properties =
                    PropertyGroup::from([property("worldViewProjection", &shape_wvp_matrix)]);
                let material = RenderMaterial::from([binding("properties", &properties)]);
                pass_stats += self.volume_marker.run_pass(
                    1,
                    &ImmutableString::new("Primary"),
                    frame_buffer,
                    bounding_shape,
                    viewport,
                    &material,
                    uniform_buffer_cache,
                    Default::default(),
                    Some(&draw_params),
                )?;
            }

            // Directional light sources cannot be optimized via occlusion culling, because they're
            // usually cover the entire screen anyway. TODO: This might still be optimizable, but
            // for now we'll skip it, since this optimization could be useful only for scenes with
            // mixed indoor/outdoor environment.
            let mut needs_lighting = true;
            if !matches!(light.kind, LightSourceKind::Directional { .. })
                && settings.use_light_occlusion_culling
            {
                if visibility_cache.needs_occlusion_query(camera_global_position, light.handle) {
                    // Draw full screen quad, that will be used to count pixels that passed the stencil test
                    // on the stencil buffer's content generated by two previous drawing commands.
                    visibility_cache.begin_query(server, camera_global_position, light.handle)?;
                    let properties =
                        PropertyGroup::from([property("worldViewProjection", &frame_matrix)]);
                    let material = RenderMaterial::from([binding("properties", &properties)]);
                    pass_stats += self.pixel_counter.run_pass(
                        1,
                        &ImmutableString::new("Primary"),
                        frame_buffer,
                        &self.quad,
                        viewport,
                        &material,
                        uniform_buffer_cache,
                        Default::default(),
                        None,
                    )?;
                    visibility_cache.end_query();
                }

                if !visibility_cache.is_visible(camera_global_position, light.handle) {
                    needs_lighting = false;
                }
            }

            if needs_lighting && shadows_enabled {
                match light.kind {
                    LightSourceKind::Spot {
                        full_cone_angle, ..
                    } => {
                        let z_near = 0.01;
                        let z_far = light_radius;
                        let light_projection_matrix =
                            Matrix4::new_perspective(1.0, full_cone_angle, z_near, z_far);

                        let light_look_at = light.position - emit_direction;

                        let light_up_vec = light
                            .look_vector
                            .try_normalize(f32::EPSILON)
                            .unwrap_or_else(Vector3::y);

                        let light_view_matrix = Matrix4::look_at_rh(
                            &Point3::from(light.position),
                            &Point3::from(light_look_at),
                            &light_up_vec,
                        );

                        light_view_projection = light_projection_matrix * light_view_matrix;

                        pass_stats += self.spot_shadow_map_renderer.render(
                            server,
                            &scene.graph,
                            elapsed_time,
                            light.position,
                            light_view_matrix,
                            z_near,
                            z_far,
                            light_projection_matrix,
                            geometry_cache,
                            cascade_index,
                            shader_cache,
                            textures,
                            fallback_resources,
                            uniform_memory_allocator,
                            dynamic_surface_cache,
                        )?;

                        light_stats.spot_shadow_maps_rendered += 1;
                    }
                    LightSourceKind::Point { .. } => {
                        pass_stats +=
                            self.point_shadow_map_renderer
                                .render(PointShadowMapRenderContext {
                                    elapsed_time,
                                    state: server,
                                    graph: &scene.graph,
                                    light_pos: light.position,
                                    light_radius,
                                    geom_cache: geometry_cache,
                                    cascade: cascade_index,
                                    shader_cache,
                                    texture_cache: textures,
                                    fallback_resources,
                                    uniform_memory_allocator,
                                    dynamic_surface_cache,
                                })?;

                        light_stats.point_shadow_maps_rendered += 1;
                    }
                    LightSourceKind::Directional { .. } => {
                        pass_stats += self.csm_renderer.render(CsmRenderContext {
                            elapsed_time,
                            frame_size: Vector2::new(gbuffer.width as f32, gbuffer.height as f32),
                            state: server,
                            graph: &scene.graph,
                            light,
                            camera,
                            geom_cache: geometry_cache,
                            shader_cache,
                            texture_cache: textures,
                            fallback_resources,
                            uniform_memory_allocator,
                            dynamic_surface_cache,
                        })?;

                        light_stats.csm_rendered += 1;
                    }
                    LightSourceKind::Unknown => {}
                }
            }

            if needs_lighting {
                let quad = &self.quad;
                let color = light.color.srgb_to_linear_f32();

                pass_stats += match light.kind {
                    LightSourceKind::Spot {
                        full_cone_angle,
                        hotspot_cone_angle,
                        shadow_bias,
                        ref cookie_texture,
                        ..
                    } => {
                        let (cookie_enabled, cookie_texture) =
                            if let Some(texture) = cookie_texture.as_ref() {
                                if let Some(cookie) = textures.get(server, texture) {
                                    (true, (&cookie.gpu_texture, &cookie.gpu_sampler))
                                } else {
                                    (
                                        false,
                                        (
                                            &fallback_resources.white_dummy,
                                            &fallback_resources.linear_wrap_sampler,
                                        ),
                                    )
                                }
                            } else {
                                (
                                    false,
                                    (
                                        &fallback_resources.white_dummy,
                                        &fallback_resources.linear_wrap_sampler,
                                    ),
                                )
                            };

                        light_stats.spot_lights_rendered += 1;

                        let inv_size = 1.0
                            / (self.spot_shadow_map_renderer.cascade_size(cascade_index) as f32);

                        let half_hotspot_cone_angle_cos = (hotspot_cone_angle * 0.5).cos();
                        let half_cone_angle_cos = (full_cone_angle * 0.5).cos();
                        let properties = PropertyGroup::from([
                            property("worldViewProjection", &frame_matrix),
                            property("lightViewProjMatrix", &light_view_projection),
                            property("invViewProj", &inv_view_projection),
                            property("lightPos", &light.position),
                            property("lightColor", &color),
                            property("cameraPosition", &camera_global_position),
                            property("lightDirection", &emit_direction),
                            property("lightRadius", &light_radius),
                            property("halfHotspotConeAngleCos", &half_hotspot_cone_angle_cos),
                            property("halfConeAngleCos", &half_cone_angle_cos),
                            property("shadowMapInvSize", &inv_size),
                            property("shadowBias", &shadow_bias),
                            property("lightIntensity", &light.intensity),
                            property("shadowAlpha", &shadows_alpha),
                            property("cookieEnabled", &cookie_enabled),
                            property("shadowsEnabled", &shadows_enabled),
                            property("softShadows", &settings.spot_soft_shadows),
                        ]);
                        let material = RenderMaterial::from([
                            binding(
                                "depthTexture",
                                (gbuffer_depth_map, &fallback_resources.nearest_clamp_sampler),
                            ),
                            binding(
                                "colorTexture",
                                (
                                    gbuffer_diffuse_map,
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "normalTexture",
                                (
                                    gbuffer_normal_map,
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "materialTexture",
                                (
                                    gbuffer_material_map,
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "spotShadowTexture",
                                (
                                    self.spot_shadow_map_renderer.cascade_texture(cascade_index),
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding("cookieTexture", cookie_texture),
                            binding("properties", &properties),
                        ]);

                        self.spot_light_shader.run_pass(
                            1,
                            &ImmutableString::new("Primary"),
                            frame_buffer,
                            quad,
                            viewport,
                            &material,
                            uniform_buffer_cache,
                            Default::default(),
                            None,
                        )?
                    }
                    LightSourceKind::Point { shadow_bias, .. } => {
                        light_stats.point_lights_rendered += 1;

                        let properties = PropertyGroup::from([
                            property("worldViewProjection", &frame_matrix),
                            property("invViewProj", &inv_view_projection),
                            property("lightPos", &light.position),
                            property("lightColor", &color),
                            property("cameraPosition", &camera_global_position),
                            property("lightRadius", &light_radius),
                            property("shadowBias", &shadow_bias),
                            property("lightIntensity", &light.intensity),
                            property("shadowAlpha", &shadows_alpha),
                            property("shadowsEnabled", &shadows_enabled),
                            property("softShadows", &settings.point_soft_shadows),
                        ]);
                        let material = RenderMaterial::from([
                            binding(
                                "depthTexture",
                                (gbuffer_depth_map, &fallback_resources.nearest_clamp_sampler),
                            ),
                            binding(
                                "colorTexture",
                                (
                                    gbuffer_diffuse_map,
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "normalTexture",
                                (
                                    gbuffer_normal_map,
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "materialTexture",
                                (
                                    gbuffer_material_map,
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "pointShadowTexture",
                                (
                                    self.point_shadow_map_renderer
                                        .cascade_texture(cascade_index),
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding("properties", &properties),
                        ]);

                        self.point_light_shader.run_pass(
                            1,
                            &ImmutableString::new("Primary"),
                            frame_buffer,
                            quad,
                            viewport,
                            &material,
                            uniform_buffer_cache,
                            Default::default(),
                            None,
                        )?
                    }
                    LightSourceKind::Directional { ref csm_options } => {
                        light_stats.directional_lights_rendered += 1;

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
                        let shadow_map_inv_size = 1.0 / (self.csm_renderer.size() as f32);
                        let shadow_bias = csm_options.shadow_bias();
                        let view_matrix = camera.view_matrix();
                        let properties = PropertyGroup::from([
                            property("worldViewProjection", &frame_matrix),
                            property("viewMatrix", &view_matrix),
                            property("invViewProj", &inv_view_projection),
                            property("lightViewProjMatrices", matrices.as_slice()),
                            property("lightColor", &color),
                            property("lightDirection", &emit_direction),
                            property("cameraPosition", &camera_global_position),
                            property("lightIntensity", &light.intensity),
                            property("shadowsEnabled", &shadows_enabled),
                            property("shadowBias", &shadow_bias),
                            property("softShadows", &settings.csm_settings.pcf),
                            property("shadowMapInvSize", &shadow_map_inv_size),
                            property("cascadeDistances", distances.as_slice()),
                        ]);
                        let cascades = self.csm_renderer.cascades();
                        let material = RenderMaterial::from([
                            binding(
                                "depthTexture",
                                (gbuffer_depth_map, &fallback_resources.nearest_clamp_sampler),
                            ),
                            binding(
                                "colorTexture",
                                (
                                    gbuffer_diffuse_map,
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "normalTexture",
                                (
                                    gbuffer_normal_map,
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "materialTexture",
                                (
                                    gbuffer_material_map,
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "shadowCascade0",
                                (
                                    cascades[0].texture(),
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "shadowCascade1",
                                (
                                    cascades[1].texture(),
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding(
                                "shadowCascade2",
                                (
                                    cascades[2].texture(),
                                    &fallback_resources.nearest_clamp_sampler,
                                ),
                            ),
                            binding("properties", &properties),
                        ]);

                        self.directional_light_shader.run_pass(
                            1,
                            &ImmutableString::new("Primary"),
                            frame_buffer,
                            quad,
                            viewport,
                            &material,
                            uniform_buffer_cache,
                            Default::default(),
                            None,
                        )?
                    }
                    LightSourceKind::Unknown => Default::default(),
                };
            }

            // Light scattering should still be renderer no matter if there's no pixels lit by the
            // light source.
            if settings.light_scatter_enabled && light.scatter_enabled {
                pass_stats += self.light_volume.render_volume(
                    light,
                    gbuffer,
                    &self.quad,
                    camera.view_matrix(),
                    inv_projection,
                    view_projection,
                    viewport,
                    &scene.graph,
                    frame_buffer,
                    uniform_buffer_cache,
                    fallback_resources,
                )?;
            }
        }

        Ok((pass_stats, light_stats))
    }
}
