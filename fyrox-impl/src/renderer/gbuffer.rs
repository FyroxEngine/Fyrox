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
    core::{algebra::Vector2, color::Color, math::Rect, sstorage::ImmutableString},
    renderer::{
        bundle::{BundleRenderContext, RenderDataBundleStorage, SurfaceInstanceData},
        cache::{
            shader::{binding, property, PropertyGroup, RenderMaterial, ShaderCache},
            uniform::{UniformBufferCache, UniformMemoryAllocator},
        },
        debug_renderer::DebugRenderer,
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, GpuFrameBuffer},
            gpu_texture::{GpuTexture, PixelKind},
            server::GraphicsServer,
        },
        observer::Observer,
        occlusion::OcclusionTester,
        resources::RendererResources,
        GeometryCache, QualitySettings, RenderPassStatistics, TextureCache,
    },
    scene::{decal::Decal, graph::Graph, mesh::RenderPath},
};
use fxhash::FxHashSet;
use fyrox_resource::manager::ResourceManager;

pub struct GBuffer {
    framebuffer: GpuFrameBuffer,
    decal_framebuffer: GpuFrameBuffer,
    pub width: i32,
    pub height: i32,

    render_pass_name: ImmutableString,
    occlusion_tester: OcclusionTester,
}

pub(crate) struct GBufferRenderContext<'a, 'b> {
    pub server: &'a dyn GraphicsServer,
    pub observer: &'b Observer,
    pub geom_cache: &'a mut GeometryCache,
    pub bundle_storage: &'a RenderDataBundleStorage,
    pub texture_cache: &'a mut TextureCache,
    pub shader_cache: &'a mut ShaderCache,
    pub renderer_resources: &'a RendererResources,
    pub quality_settings: &'a QualitySettings,
    pub graph: &'b Graph,
    pub uniform_buffer_cache: &'a mut UniformBufferCache,
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,
    #[allow(dead_code)]
    pub screen_space_debug_renderer: &'a mut DebugRenderer,
    pub resource_manager: &'a ResourceManager,
}

impl GBuffer {
    pub fn new(
        server: &dyn GraphicsServer,
        width: usize,
        height: usize,
    ) -> Result<Self, FrameworkError> {
        let diffuse_texture = server.create_2d_render_target(
            "GBufferDiffuseTexture",
            PixelKind::RGBA8,
            width,
            height,
        )?;
        let normal_texture = server.create_2d_render_target(
            "GBufferNormalTexture",
            PixelKind::RGBA8,
            width,
            height,
        )?;
        let framebuffer = server.create_frame_buffer(
            Some(Attachment::depth_stencil(server.create_2d_render_target(
                "GBufferDepthStencilTexture",
                PixelKind::D24S8,
                width,
                height,
            )?)),
            vec![
                Attachment::color(diffuse_texture.clone()),
                Attachment::color(normal_texture.clone()),
                Attachment::color(server.create_2d_render_target(
                    "GBufferAmbientTexture",
                    PixelKind::RGBA16F,
                    width,
                    height,
                )?),
                Attachment::color(server.create_2d_render_target(
                    "GBufferMaterialTexture",
                    PixelKind::RGBA8,
                    width,
                    height,
                )?),
                Attachment::color(server.create_2d_render_target(
                    "GBufferDecalMaskTexture",
                    PixelKind::R8UI,
                    width,
                    height,
                )?),
            ],
        )?;

        let decal_framebuffer = server.create_frame_buffer(
            None,
            vec![
                Attachment::color(diffuse_texture),
                Attachment::color(normal_texture),
            ],
        )?;

        Ok(Self {
            framebuffer,
            width: width as i32,
            height: height as i32,
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
            observer,
            geom_cache,
            bundle_storage,
            texture_cache,
            shader_cache,
            quality_settings,
            renderer_resources,
            graph,
            uniform_buffer_cache,
            uniform_memory_allocator,
            resource_manager,
            ..
        } = args;

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
            .cell(observer.position.translation);

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
                resource_manager,
                use_pom: quality_settings.use_parallax_mapping,
                light_position: &Default::default(),
                renderer_resources,
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
                objects.iter(),
                &self.framebuffer,
                observer.position.translation,
                observer.position.view_projection_matrix,
                uniform_buffer_cache,
                renderer_resources,
            )?;
        }

        let inv_view_proj = observer
            .position
            .view_projection_matrix
            .try_inverse()
            .unwrap_or_default();
        let depth = self.depth();
        let decal_mask = self.decal_mask_texture();
        let resolution = Vector2::new(self.width as f32, self.height as f32);

        // Render decals after because we need to modify diffuse texture of G-Buffer and use depth texture
        // for rendering. We'll render in the G-Buffer, but depth will be used from final frame, since
        // decals do not modify depth (only diffuse and normal maps).
        for decal in graph.linear_iter().filter_map(|n| n.cast::<Decal>()) {
            let world_view_proj =
                observer.position.view_projection_matrix * decal.global_transform();

            let diffuse_texture = decal
                .diffuse_texture()
                .and_then(|t| {
                    texture_cache
                        .get(server, resource_manager, t)
                        .map(|t| (t.gpu_texture.clone(), t.gpu_sampler.clone()))
                })
                .unwrap_or((
                    renderer_resources.white_dummy.clone(),
                    renderer_resources.linear_clamp_sampler.clone(),
                ))
                .clone();

            let normal_texture = decal
                .normal_texture()
                .and_then(|t| {
                    texture_cache
                        .get(server, resource_manager, t)
                        .map(|t| (t.gpu_texture.clone(), t.gpu_sampler.clone()))
                })
                .unwrap_or((
                    renderer_resources.normal_dummy.clone(),
                    renderer_resources.linear_clamp_sampler.clone(),
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
                    (depth, &renderer_resources.nearest_clamp_sampler),
                ),
                binding("diffuseTexture", (&diffuse_texture.0, &diffuse_texture.1)),
                binding("normalTexture", (&normal_texture.0, &normal_texture.1)),
                binding(
                    "decalMask",
                    (decal_mask, &renderer_resources.nearest_clamp_sampler),
                ),
                binding("properties", &properties),
            ]);

            statistics += renderer_resources.shaders.decal.run_pass(
                1,
                &ImmutableString::new("Primary"),
                &self.decal_framebuffer,
                &renderer_resources.cube,
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
