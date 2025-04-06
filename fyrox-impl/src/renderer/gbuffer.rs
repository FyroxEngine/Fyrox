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

//! GBuffer Layout:
//!
//! RT0: sRGBA8 - Diffuse color (xyz)
//! RT1: RGBA8 - Normal (xyz)
//! RT2: RGBA16F - Ambient light + emission (both in xyz)
//! RT3: RGBA8 - Metallic (x) + Roughness (y) + Ambient Occlusion (z)
//! RT4: R8UI - Decal mask (x)
//!
//! Every alpha channel is used for layer blending for terrains. This is inefficient, but for
//! now I don't know better solution.

use crate::{
    core::{
        algebra::{Matrix4, Vector2},
        color::Color,
        math::Rect,
        sstorage::ImmutableString,
    },
    renderer::{
        bundle::{BundleRenderContext, RenderDataBundleStorage, SurfaceInstanceData},
        cache::{
            shader::{
                binding, property, PropertyGroup, RenderMaterial, RenderPassContainer, ShaderCache,
            },
            uniform::{UniformBufferCache, UniformMemoryAllocator},
        },
        debug_renderer::DebugRenderer,
        framework::{
            buffer::BufferUsage,
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind, GpuFrameBuffer},
            geometry_buffer::GpuGeometryBuffer,
            gpu_texture::{GpuTexture, PixelKind},
            server::GraphicsServer,
            GeometryBufferExt,
        },
        occlusion::OcclusionTester,
        FallbackResources, GeometryCache, QualitySettings, RenderPassStatistics, TextureCache,
    },
    scene::{
        camera::Camera,
        decal::Decal,
        graph::Graph,
        mesh::{surface::SurfaceData, RenderPath},
    },
};
use fxhash::FxHashSet;

pub struct GBuffer {
    framebuffer: GpuFrameBuffer,
    decal_framebuffer: GpuFrameBuffer,
    pub width: i32,
    pub height: i32,
    cube: GpuGeometryBuffer,
    decal_shader: RenderPassContainer,
    render_pass_name: ImmutableString,
    occlusion_tester: OcclusionTester,
}

pub(crate) struct GBufferRenderContext<'a, 'b> {
    pub server: &'a dyn GraphicsServer,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub bundle_storage: &'a RenderDataBundleStorage,
    pub texture_cache: &'a mut TextureCache,
    pub shader_cache: &'a mut ShaderCache,
    pub fallback_resources: &'a FallbackResources,
    pub quality_settings: &'a QualitySettings,
    pub graph: &'b Graph,
    pub uniform_buffer_cache: &'a mut UniformBufferCache,
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,
    #[allow(dead_code)]
    pub screen_space_debug_renderer: &'a mut DebugRenderer,
    pub unit_quad: &'a GpuGeometryBuffer,
}

impl GBuffer {
    pub fn new(
        server: &dyn GraphicsServer,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        let diffuse_texture = server.create_2d_render_target(PixelKind::RGBA8, width, height)?;
        let normal_texture = server.create_2d_render_target(PixelKind::RGBA8, width, height)?;
        let framebuffer = server.create_frame_buffer(
            Some(Attachment {
                kind: AttachmentKind::DepthStencil,
                texture: server.create_2d_render_target(PixelKind::D24S8, width, height)?,
            }),
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: diffuse_texture.clone(),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: normal_texture.clone(),
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: server.create_2d_render_target(PixelKind::RGBA16F, width, height)?,
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: server.create_2d_render_target(PixelKind::RGBA8, width, height)?,
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: server.create_2d_render_target(PixelKind::R8UI, width, height)?,
                },
            ],
        )?;

        let decal_framebuffer = server.create_frame_buffer(
            None,
            vec![
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: diffuse_texture,
                },
                Attachment {
                    kind: AttachmentKind::Color,
                    texture: normal_texture,
                },
            ],
        )?;

        Ok(Self {
            framebuffer,
            width: width as i32,
            height: height as i32,
            decal_shader: RenderPassContainer::from_str(
                server,
                include_str!("shaders/decal.shader"),
            )?,
            cube: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_cube(Matrix4::identity()),
                BufferUsage::StaticDraw,
                server,
            )?,
            decal_framebuffer,
            render_pass_name: ImmutableString::new("GBuffer"),
            occlusion_tester: OcclusionTester::new(server, width, height, 16)?,
        })
    }

    pub fn framebuffer(&self) -> &GpuFrameBuffer {
        &self.framebuffer
    }

    pub fn depth(&self) -> &GpuTexture {
        &self.framebuffer.depth_attachment().unwrap().texture
    }

    pub fn diffuse_texture(&self) -> &GpuTexture {
        &self.framebuffer.color_attachments()[0].texture
    }

    pub fn normal_texture(&self) -> &GpuTexture {
        &self.framebuffer.color_attachments()[1].texture
    }

    pub fn ambient_texture(&self) -> &GpuTexture {
        &self.framebuffer.color_attachments()[2].texture
    }

    pub fn material_texture(&self) -> &GpuTexture {
        &self.framebuffer.color_attachments()[3].texture
    }

    pub fn decal_mask_texture(&self) -> &GpuTexture {
        &self.framebuffer.color_attachments()[4].texture
    }

    pub(crate) fn fill(
        &mut self,
        args: GBufferRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

        let GBufferRenderContext {
            server,
            camera,
            geom_cache,
            bundle_storage,
            texture_cache,
            shader_cache,
            quality_settings,
            fallback_resources,
            graph,
            uniform_buffer_cache,
            unit_quad,
            uniform_memory_allocator,
            ..
        } = args;

        let view_projection = camera.view_projection_matrix();

        if quality_settings.use_occlusion_culling {
            self.occlusion_tester.try_query_visibility_results(graph);
        };

        let viewport = Rect::new(0, 0, self.width, self.height);
        self.framebuffer.clear(
            viewport,
            Some(Color::from_rgba(0, 0, 0, 0)),
            Some(1.0),
            Some(0),
        );

        let grid_cell = self
            .occlusion_tester
            .grid_cache
            .cell(camera.global_position());

        let instance_filter = |instance: &SurfaceInstanceData| {
            !quality_settings.use_occlusion_culling
                || grid_cell.is_none_or(|cell| cell.is_visible(instance.node_handle))
        };

        statistics += bundle_storage.render_to_frame_buffer(
            server,
            geom_cache,
            shader_cache,
            |bundle| bundle.render_path == RenderPath::Deferred,
            instance_filter,
            BundleRenderContext {
                texture_cache,
                render_pass_name: &self.render_pass_name,
                frame_buffer: &self.framebuffer,
                viewport,
                uniform_memory_allocator,
                use_pom: quality_settings.use_parallax_mapping,
                light_position: &Default::default(),
                fallback_resources,
                ambient_light: Color::WHITE, // TODO
                scene_depth: None,           // TODO. Add z-pre-pass.
            },
        )?;

        if quality_settings.use_occlusion_culling {
            let mut objects = FxHashSet::default();
            for bundle in bundle_storage.bundles.iter() {
                for instance in bundle.instances.iter() {
                    objects.insert(instance.node_handle);
                }
            }

            self.occlusion_tester.try_run_visibility_test(
                graph,
                None,
                unit_quad,
                objects.iter(),
                &self.framebuffer,
                camera.global_position(),
                view_projection,
                uniform_buffer_cache,
                fallback_resources,
            )?;
        }

        let inv_view_proj = view_projection.try_inverse().unwrap_or_default();
        let depth = self.depth();
        let decal_mask = self.decal_mask_texture();
        let resolution = Vector2::new(self.width as f32, self.height as f32);

        // Render decals after because we need to modify diffuse texture of G-Buffer and use depth texture
        // for rendering. We'll render in the G-Buffer, but depth will be used from final frame, since
        // decals do not modify depth (only diffuse and normal maps).
        let unit_cube = &self.cube;
        for decal in graph.linear_iter().filter_map(|n| n.cast::<Decal>()) {
            let world_view_proj = view_projection * decal.global_transform();

            let diffuse_texture = decal
                .diffuse_texture()
                .and_then(|t| {
                    texture_cache
                        .get(server, t)
                        .map(|t| (t.gpu_texture.clone(), t.gpu_sampler.clone()))
                })
                .unwrap_or((
                    fallback_resources.white_dummy.clone(),
                    fallback_resources.linear_clamp_sampler.clone(),
                ))
                .clone();

            let normal_texture = decal
                .normal_texture()
                .and_then(|t| {
                    texture_cache
                        .get(server, t)
                        .map(|t| (t.gpu_texture.clone(), t.gpu_sampler.clone()))
                })
                .unwrap_or((
                    fallback_resources.normal_dummy.clone(),
                    fallback_resources.linear_clamp_sampler.clone(),
                ))
                .clone();

            let inv_world_decal = decal.global_transform().try_inverse().unwrap_or_default();
            let color = decal.color().srgb_to_linear_f32();
            let layer_index = decal.layer() as u32;
            let properties = PropertyGroup::from([
                property("worldViewProjection", &world_view_proj),
                property("invViewProj", &inv_view_proj),
                property("invWorldDecal", &inv_world_decal),
                property("resolution", &resolution),
                property("color", &color),
                property("layerIndex", &layer_index),
            ]);
            let material = RenderMaterial::from([
                binding(
                    "sceneDepth",
                    (depth, &fallback_resources.nearest_clamp_sampler),
                ),
                binding("diffuseTexture", (&diffuse_texture.0, &diffuse_texture.1)),
                binding("normalTexture", (&normal_texture.0, &normal_texture.1)),
                binding(
                    "decalMask",
                    (decal_mask, &fallback_resources.nearest_clamp_sampler),
                ),
                binding("properties", &properties),
            ]);

            statistics += self.decal_shader.run_pass(
                1,
                &ImmutableString::new("Primary"),
                &self.decal_framebuffer,
                unit_cube,
                viewport,
                &material,
                uniform_buffer_cache,
                Default::default(),
                None,
            )?;
        }

        Ok(statistics)
    }
}
