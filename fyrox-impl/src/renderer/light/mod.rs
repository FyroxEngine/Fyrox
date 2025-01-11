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

use crate::renderer::make_viewport_matrix;
use crate::{
    core::{
        algebra::{Matrix4, Point3, UnitQuaternion, Vector2, Vector3},
        color::Color,
        math::{frustum::Frustum, Matrix4Ext, Rect, TriangleDefinition},
    },
    renderer::{
        bundle::{LightSourceKind, RenderDataBundleStorage},
        cache::{
            shader::ShaderCache, uniform::UniformBufferCache, uniform::UniformMemoryAllocator,
        },
        flat_shader::FlatShader,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{FrameBuffer, ResourceBindGroup, ResourceBinding},
            geometry_buffer::GeometryBuffer,
            server::GraphicsServer,
            uniform::StaticUniformBuffer,
            BlendFactor, BlendFunc, BlendParameters, ColorMask, CompareFunc, CullFace,
            DrawParameters, ElementRange, GeometryBufferExt, StencilAction, StencilFunc, StencilOp,
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
use fyrox_graphics::framebuffer::BufferLocation;

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
    quad: Box<dyn GeometryBuffer>,
    sphere: Box<dyn GeometryBuffer>,
    cone: Box<dyn GeometryBuffer>,
    skybox: Box<dyn GeometryBuffer>,
    flat_shader: FlatShader,
    skybox_shader: SkyboxShader,
    spot_shadow_map_renderer: SpotShadowMapRenderer,
    point_shadow_map_renderer: PointShadowMapRenderer,
    csm_renderer: CsmRenderer,
    light_volume: LightVolumeRenderer,
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
    pub frame_buffer: &'a mut dyn FrameBuffer,
    pub shader_cache: &'a mut ShaderCache,
    pub fallback_resources: &'a FallbackResources,
    pub uniform_buffer_cache: &'a mut UniformBufferCache,
    pub visibility_cache: &'a mut ObserverVisibilityCache,
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,
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
            spot_light_shader: SpotLightShader::new(server)?,
            point_light_shader: PointLightShader::new(server)?,
            directional_light_shader: DirectionalLightShader::new(server)?,
            ambient_light_shader: AmbientLightShader::new(server)?,
            quad: <dyn GeometryBuffer>::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )?,
            skybox: <dyn GeometryBuffer>::from_surface_data(
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
            sphere: <dyn GeometryBuffer>::from_surface_data(
                &SurfaceData::make_sphere(10, 10, 1.0, &Matrix4::identity()),
                BufferUsage::StaticDraw,
                server,
            )?,
            cone: <dyn GeometryBuffer>::from_surface_data(
                &SurfaceData::make_cone(
                    16,
                    0.5,
                    1.0,
                    &Matrix4::new_translation(&Vector3::new(0.0, -1.0, 0.0)),
                ),
                BufferUsage::StaticDraw,
                server,
            )?,
            flat_shader: FlatShader::new(server)?,
            skybox_shader: SkyboxShader::new(server)?,
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
            )?;
        }

        // Render skybox (if any).
        if let Some(skybox) = camera.skybox_ref() {
            let size = camera.projection().z_far() / 2.0f32.sqrt();
            let scale = Matrix4::new_scaling(size);
            let wvp = Matrix4::new_translation(&camera.global_position()) * scale;

            if let Some(gpu_texture) = skybox
                .cubemap_ref()
                .and_then(|cube_map| textures.get(server, cube_map))
            {
                let shader = &self.skybox_shader;
                pass_stats += frame_buffer.draw(
                    &*self.skybox,
                    viewport,
                    &*shader.program,
                    &DrawParameters {
                        cull_face: None,
                        color_write: Default::default(),
                        depth_write: false,
                        stencil_test: None,
                        depth_test: None,
                        blend: None,
                        stencil_op: Default::default(),
                        scissor_box: None,
                    },
                    &[ResourceBindGroup {
                        bindings: &[
                            ResourceBinding::texture(gpu_texture, &shader.cubemap_texture),
                            ResourceBinding::Buffer {
                                buffer: uniform_buffer_cache.write(
                                    StaticUniformBuffer::<256>::new()
                                        .with(&(view_projection * wvp)),
                                )?,
                                binding: BufferLocation::Auto {
                                    shader_location: shader.uniform_buffer_binding,
                                },
                                data_usage: Default::default(),
                            },
                        ],
                    }],
                    ElementRange::Specific {
                        offset: 0,
                        count: 12,
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
            &*self.quad,
            viewport,
            &*self.ambient_light_shader.program,
            &DrawParameters {
                cull_face: None,
                color_write: Default::default(),
                depth_write: false,
                stencil_test: None,
                depth_test: None,
                blend: Some(BlendParameters {
                    func: BlendFunc::new(BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
                    ..Default::default()
                }),
                stencil_op: Default::default(),
                scissor_box: None,
            },
            &[ResourceBindGroup {
                bindings: &[
                    ResourceBinding::texture(
                        &gbuffer_diffuse_map,
                        &self.ambient_light_shader.diffuse_texture,
                    ),
                    ResourceBinding::texture(
                        if settings.use_ssao {
                            &ao_map
                        } else {
                            &fallback_resources.white_dummy
                        },
                        &self.ambient_light_shader.ao_sampler,
                    ),
                    ResourceBinding::texture(
                        &gbuffer_ambient_map,
                        &self.ambient_light_shader.ambient_texture,
                    ),
                    ResourceBinding::Buffer {
                        buffer: uniform_buffer_cache.write(
                            StaticUniformBuffer::<256>::new()
                                .with(&frame_matrix)
                                .with(&ambient_color.srgb_to_linear_f32()),
                        )?,
                        binding: BufferLocation::Auto {
                            shader_location: self.ambient_light_shader.uniform_buffer_binding,
                        },
                        data_usage: Default::default(),
                    },
                ],
            }],
            ElementRange::Full,
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
            let uniform_buffer = uniform_buffer_cache.write(
                StaticUniformBuffer::<256>::new().with(&(view_projection * bounding_shape_matrix)),
            )?;

            for (cull_face, stencil_action) in [
                (CullFace::Front, StencilAction::Incr),
                (CullFace::Back, StencilAction::Decr),
            ] {
                pass_stats += frame_buffer.draw(
                    &**bounding_shape,
                    viewport,
                    &*self.flat_shader.program,
                    &DrawParameters {
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
                    },
                    &[ResourceBindGroup {
                        bindings: &[ResourceBinding::Buffer {
                            buffer: uniform_buffer,
                            binding: BufferLocation::Auto {
                                shader_location: self.flat_shader.uniform_buffer_binding,
                            },
                            data_usage: Default::default(),
                        }],
                    }],
                    ElementRange::Full,
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
                    let uniform_buffer = uniform_buffer_cache
                        .write(StaticUniformBuffer::<256>::new().with(&frame_matrix))?;

                    visibility_cache.begin_query(server, camera_global_position, light.handle)?;
                    frame_buffer.draw(
                        &*self.quad,
                        viewport,
                        &*self.flat_shader.program,
                        &DrawParameters {
                            cull_face: None,
                            color_write: ColorMask::all(false),
                            depth_write: false,
                            stencil_test: Some(StencilFunc {
                                func: CompareFunc::NotEqual,
                                ..Default::default()
                            }),
                            depth_test: None,
                            blend: None,
                            stencil_op: Default::default(),
                            scissor_box: None,
                        },
                        &[ResourceBindGroup {
                            bindings: &[ResourceBinding::Buffer {
                                buffer: uniform_buffer,
                                binding: BufferLocation::Auto {
                                    shader_location: self.flat_shader.uniform_buffer_binding,
                                },
                                data_usage: Default::default(),
                            }],
                        }],
                        ElementRange::Full,
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
                        })?;

                        light_stats.csm_rendered += 1;
                    }
                    LightSourceKind::Unknown => {}
                }
            }

            if needs_lighting {
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
                    depth_test: None,
                    blend: Some(BlendParameters {
                        func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                        ..Default::default()
                    }),
                    scissor_box: None,
                };

                let quad = &self.quad;

                pass_stats += match light.kind {
                    LightSourceKind::Spot {
                        full_cone_angle,
                        hotspot_cone_angle,
                        shadow_bias,
                        ref cookie_texture,
                        ..
                    } => {
                        let shader = &self.spot_light_shader;

                        let (cookie_enabled, cookie_texture) =
                            if let Some(texture) = cookie_texture.as_ref() {
                                if let Some(cookie) = textures.get(server, texture) {
                                    (true, cookie)
                                } else {
                                    (false, &fallback_resources.white_dummy)
                                }
                            } else {
                                (false, &fallback_resources.white_dummy)
                            };

                        light_stats.spot_lights_rendered += 1;

                        let inv_size = 1.0
                            / (self.spot_shadow_map_renderer.cascade_size(cascade_index) as f32);
                        let uniform_buffer = uniform_buffer_cache.write(
                            StaticUniformBuffer::<1024>::new()
                                .with(&frame_matrix)
                                .with(&light_view_projection)
                                .with(&inv_view_projection)
                                .with(&light.position)
                                .with(&light.color.srgb_to_linear_f32())
                                .with(&camera_global_position)
                                .with(&emit_direction)
                                .with(&light_radius)
                                .with(&(hotspot_cone_angle * 0.5).cos())
                                .with(&(full_cone_angle * 0.5).cos())
                                .with(&inv_size)
                                .with(&shadow_bias)
                                .with(&light.intensity)
                                .with(&shadows_alpha)
                                .with(&cookie_enabled)
                                .with(&shadows_enabled)
                                .with(&settings.spot_soft_shadows),
                        )?;

                        frame_buffer.draw(
                            &**quad,
                            viewport,
                            &*shader.program,
                            &draw_params,
                            &[ResourceBindGroup {
                                bindings: &[
                                    ResourceBinding::texture(
                                        &gbuffer_depth_map,
                                        &shader.depth_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &gbuffer_diffuse_map,
                                        &shader.color_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &gbuffer_normal_map,
                                        &shader.normal_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &gbuffer_material_map,
                                        &shader.material_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &self
                                            .spot_shadow_map_renderer
                                            .cascade_texture(cascade_index),
                                        &shader.spot_shadow_texture,
                                    ),
                                    ResourceBinding::texture(
                                        cookie_texture,
                                        &shader.cookie_texture,
                                    ),
                                    ResourceBinding::Buffer {
                                        buffer: uniform_buffer,
                                        binding: BufferLocation::Auto {
                                            shader_location: shader.uniform_buffer_binding,
                                        },
                                        data_usage: Default::default(),
                                    },
                                ],
                            }],
                            ElementRange::Full,
                        )?
                    }
                    LightSourceKind::Point { shadow_bias, .. } => {
                        let shader = &self.point_light_shader;

                        light_stats.point_lights_rendered += 1;

                        let uniform_buffer = uniform_buffer_cache.write(
                            StaticUniformBuffer::<1024>::new()
                                .with(&frame_matrix)
                                .with(&inv_view_projection)
                                .with(&light.color.srgb_to_linear_f32())
                                .with(&light.position)
                                .with(&camera_global_position)
                                .with(&light_radius)
                                .with(&shadow_bias)
                                .with(&light.intensity)
                                .with(&shadows_alpha)
                                .with(&settings.point_soft_shadows)
                                .with(&shadows_enabled),
                        )?;

                        frame_buffer.draw(
                            &**quad,
                            viewport,
                            &*shader.program,
                            &draw_params,
                            &[ResourceBindGroup {
                                bindings: &[
                                    ResourceBinding::texture(
                                        &gbuffer_depth_map,
                                        &shader.depth_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &gbuffer_diffuse_map,
                                        &shader.color_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &gbuffer_normal_map,
                                        &shader.normal_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &gbuffer_material_map,
                                        &shader.material_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &self
                                            .point_shadow_map_renderer
                                            .cascade_texture(cascade_index),
                                        &shader.point_shadow_texture,
                                    ),
                                    ResourceBinding::Buffer {
                                        buffer: uniform_buffer,
                                        binding: BufferLocation::Auto {
                                            shader_location: shader.uniform_buffer_binding,
                                        },
                                        data_usage: Default::default(),
                                    },
                                ],
                            }],
                            ElementRange::Full,
                        )?
                    }
                    LightSourceKind::Directional { ref csm_options } => {
                        let shader = &self.directional_light_shader;

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

                        let uniform_buffer = uniform_buffer_cache.write(
                            StaticUniformBuffer::<1024>::new()
                                .with(&frame_matrix)
                                .with(&camera.view_matrix())
                                .with(&inv_view_projection)
                                .with_slice(&matrices)
                                .with(&light.color.srgb_to_linear_f32())
                                .with(&emit_direction)
                                .with(&camera_global_position)
                                .with(&light.intensity)
                                .with(&shadows_enabled)
                                .with(&csm_options.shadow_bias())
                                .with(&settings.csm_settings.pcf)
                                .with(&(1.0 / (self.csm_renderer.size() as f32)))
                                .with_slice(&distances),
                        )?;

                        frame_buffer.draw(
                            &**quad,
                            viewport,
                            &*shader.program,
                            &DrawParameters {
                                cull_face: None,
                                color_write: Default::default(),
                                depth_write: false,
                                stencil_test: None,
                                depth_test: None,
                                blend: Some(BlendParameters {
                                    func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                                    ..Default::default()
                                }),
                                stencil_op: Default::default(),
                                scissor_box: None,
                            },
                            &[ResourceBindGroup {
                                bindings: &[
                                    ResourceBinding::texture(
                                        &gbuffer_depth_map,
                                        &shader.depth_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &gbuffer_diffuse_map,
                                        &shader.color_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &gbuffer_normal_map,
                                        &shader.normal_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &gbuffer_material_map,
                                        &shader.material_sampler,
                                    ),
                                    ResourceBinding::texture(
                                        &self.csm_renderer.cascades()[0].texture(),
                                        &shader.shadow_cascade0,
                                    ),
                                    ResourceBinding::texture(
                                        &self.csm_renderer.cascades()[1].texture(),
                                        &shader.shadow_cascade1,
                                    ),
                                    ResourceBinding::texture(
                                        &self.csm_renderer.cascades()[2].texture(),
                                        &shader.shadow_cascade2,
                                    ),
                                    ResourceBinding::Buffer {
                                        buffer: uniform_buffer,
                                        binding: BufferLocation::Auto {
                                            shader_location: shader.uniform_buffer_binding,
                                        },
                                        data_usage: Default::default(),
                                    },
                                ],
                            }],
                            ElementRange::Full,
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
                    &*self.quad,
                    camera.view_matrix(),
                    inv_projection,
                    view_projection,
                    viewport,
                    &scene.graph,
                    frame_buffer,
                    uniform_buffer_cache,
                )?;
            }
        }

        Ok((pass_stats, light_stats))
    }
}
