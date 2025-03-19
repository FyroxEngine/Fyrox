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
        algebra::{Matrix4, Vector3},
        color::Color,
        math::Rect,
    },
    renderer::{
        bundle::{
            BundleRenderContext, ObserverInfo, RenderDataBundleStorage,
            RenderDataBundleStorageOptions,
        },
        cache::{shader::ShaderCache, texture::TextureCache, uniform::UniformMemoryAllocator},
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind},
            gpu_texture::PixelKind,
            server::GraphicsServer,
        },
        shadow::cascade_size,
        FallbackResources, GeometryCache, RenderPassStatistics, ShadowMapPrecision,
        SPOT_SHADOW_PASS_NAME,
    },
    scene::graph::Graph,
};
use fyrox_graphics::framebuffer::GpuFrameBuffer;
use fyrox_graphics::gpu_texture::GpuTexture;

pub struct SpotShadowMapRenderer {
    precision: ShadowMapPrecision,
    // Three "cascades" for various use cases:
    //  0 - largest, for lights close to camera.
    //  1 - medium, for lights with medium distance to camera.
    //  2 - small, for farthest lights.
    cascades: [GpuFrameBuffer; 3],
    size: usize,
}

impl SpotShadowMapRenderer {
    pub fn new(
        server: &dyn GraphicsServer,
        size: usize,
        precision: ShadowMapPrecision,
    ) -> Result<Self, FrameworkError> {
        fn make_cascade(
            server: &dyn GraphicsServer,
            size: usize,
            precision: ShadowMapPrecision,
        ) -> Result<GpuFrameBuffer, FrameworkError> {
            let depth = server.create_2d_render_target(
                match precision {
                    ShadowMapPrecision::Full => PixelKind::D32F,
                    ShadowMapPrecision::Half => PixelKind::D16,
                },
                size,
                size,
            )?;

            server.create_frame_buffer(
                Some(Attachment {
                    kind: AttachmentKind::Depth,
                    texture: depth,
                }),
                vec![],
            )
        }

        Ok(Self {
            precision,
            size,
            cascades: [
                make_cascade(server, cascade_size(size, 0), precision)?,
                make_cascade(server, cascade_size(size, 1), precision)?,
                make_cascade(server, cascade_size(size, 2), precision)?,
            ],
        })
    }

    pub fn base_size(&self) -> usize {
        self.size
    }

    pub fn precision(&self) -> ShadowMapPrecision {
        self.precision
    }

    pub fn cascade_texture(&self, cascade: usize) -> &GpuTexture {
        &self.cascades[cascade].depth_attachment().unwrap().texture
    }

    pub fn cascade_size(&self, cascade: usize) -> usize {
        cascade_size(self.size, cascade)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render(
        &mut self,
        server: &dyn GraphicsServer,
        graph: &Graph,
        elapsed_time: f32,
        light_position: Vector3<f32>,
        light_view_matrix: Matrix4<f32>,
        z_near: f32,
        z_far: f32,
        light_projection_matrix: Matrix4<f32>,
        geom_cache: &mut GeometryCache,
        cascade: usize,
        shader_cache: &mut ShaderCache,
        texture_cache: &mut TextureCache,
        fallback_resources: &FallbackResources,
        uniform_memory_allocator: &mut UniformMemoryAllocator,
        dynamic_surface_cache: &mut DynamicSurfaceCache,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

        let framebuffer = &self.cascades[cascade];
        let cascade_size = cascade_size(self.size, cascade);

        let viewport = Rect::new(0, 0, cascade_size as i32, cascade_size as i32);

        framebuffer.clear(viewport, None, Some(1.0), None);

        let bundle_storage = RenderDataBundleStorage::from_graph(
            graph,
            elapsed_time,
            ObserverInfo {
                observer_position: light_position,
                z_near,
                z_far,
                view_matrix: light_view_matrix,
                projection_matrix: light_projection_matrix,
            },
            SPOT_SHADOW_PASS_NAME.clone(),
            RenderDataBundleStorageOptions {
                collect_lights: false,
            },
            dynamic_surface_cache,
        );

        statistics += bundle_storage.render_to_frame_buffer(
            server,
            geom_cache,
            shader_cache,
            |_| true,
            |_| true,
            BundleRenderContext {
                texture_cache,
                render_pass_name: &SPOT_SHADOW_PASS_NAME,
                frame_buffer: framebuffer,
                viewport,
                uniform_memory_allocator,
                use_pom: false,
                light_position: &Default::default(),
                fallback_resources,
                ambient_light: Color::WHITE, // TODO
                scene_depth: None,
            },
        )?;

        Ok(statistics)
    }
}
