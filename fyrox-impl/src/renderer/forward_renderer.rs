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

//! Forward renderer is used to render transparent meshes and meshes with custom blending options.

use crate::{
    core::{color::Color, math::Rect, sstorage::ImmutableString},
    renderer::{
        bundle::{BundleRenderContext, RenderDataBundleStorage},
        cache::{shader::ShaderCache, texture::TextureCache, uniform::UniformMemoryAllocator},
        framework::{error::FrameworkError, server::GraphicsServer},
        FallbackResources, GeometryCache, QualitySettings, RenderPassStatistics,
    },
    scene::mesh::RenderPath,
};
use fyrox_graphics::framebuffer::GpuFrameBuffer;
use fyrox_graphics::gpu_texture::GpuTexture;

pub(crate) struct ForwardRenderer {
    render_pass_name: ImmutableString,
}

pub(crate) struct ForwardRenderContext<'a> {
    pub state: &'a dyn GraphicsServer,
    pub geom_cache: &'a mut GeometryCache,
    pub texture_cache: &'a mut TextureCache,
    pub shader_cache: &'a mut ShaderCache,
    pub bundle_storage: &'a RenderDataBundleStorage,
    pub framebuffer: &'a GpuFrameBuffer,
    pub viewport: Rect<i32>,
    pub quality_settings: &'a QualitySettings,
    pub fallback_resources: &'a FallbackResources,
    pub scene_depth: &'a GpuTexture,
    pub ambient_light: Color,
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,
}

impl ForwardRenderer {
    pub(crate) fn new() -> Self {
        Self {
            render_pass_name: ImmutableString::new("Forward"),
        }
    }

    pub(crate) fn render(
        &self,
        args: ForwardRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut statistics = RenderPassStatistics::default();

        let ForwardRenderContext {
            state,
            geom_cache,
            texture_cache,
            shader_cache,
            bundle_storage,
            framebuffer,
            viewport,
            quality_settings,
            fallback_resources,
            scene_depth,
            ambient_light,
            uniform_memory_allocator,
        } = args;

        statistics += bundle_storage.render_to_frame_buffer(
            state,
            geom_cache,
            shader_cache,
            |bundle| bundle.render_path == RenderPath::Forward,
            |_| true,
            BundleRenderContext {
                texture_cache,
                render_pass_name: &self.render_pass_name,
                frame_buffer: framebuffer,
                viewport,
                uniform_memory_allocator,
                use_pom: quality_settings.use_parallax_mapping,
                light_position: &Default::default(),
                fallback_resources,
                ambient_light,
                scene_depth: Some(scene_depth),
            },
        )?;

        Ok(statistics)
    }
}
