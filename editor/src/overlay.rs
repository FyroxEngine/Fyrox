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

use crate::{
    fyrox::{
        core::{algebra::Matrix4, math::Matrix4Ext, sstorage::ImmutableString},
        renderer::{
            cache::shader::{
                binding, property, PropertyGroup, RenderMaterial, RenderPassContainer,
            },
            framework::{
                buffer::BufferUsage, error::FrameworkError, geometry_buffer::GpuGeometryBuffer,
                server::GraphicsServer, GeometryBufferExt,
            },
            RenderPassStatistics, SceneRenderPass, SceneRenderPassContext,
        },
        resource::texture::{
            CompressionOptions, TextureImportOptions, TextureMinificationFilter, TextureResource,
            TextureResourceExtension,
        },
        scene::mesh::surface::SurfaceData,
    },
    Editor,
};
use fyrox::asset::untyped::ResourceKind;
use fyrox::core::Uuid;
use std::{any::TypeId, cell::RefCell, rc::Rc};

pub struct OverlayRenderPass {
    quad: GpuGeometryBuffer,
    shader: RenderPassContainer,
    sound_icon: TextureResource,
    light_icon: TextureResource,
    pub pictogram_size: f32,
}

impl OverlayRenderPass {
    pub fn new(server: &dyn GraphicsServer) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            quad: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_collapsed_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )
            .unwrap(),
            shader: RenderPassContainer::from_str(
                server,
                include_str!("../resources/shaders/overlay.shader"),
            )
            .unwrap(),
            sound_icon: TextureResource::load_from_memory(
                Uuid::new_v4(),
                ResourceKind::Embedded,
                include_bytes!("../resources/sound_source.png"),
                TextureImportOptions::default()
                    .with_compression(CompressionOptions::NoCompression)
                    .with_minification_filter(TextureMinificationFilter::Linear),
            )
            .unwrap(),
            light_icon: TextureResource::load_from_memory(
                Uuid::new_v4(),
                ResourceKind::Embedded,
                include_bytes!("../resources/light_source.png"),
                TextureImportOptions::default()
                    .with_compression(CompressionOptions::NoCompression)
                    .with_minification_filter(TextureMinificationFilter::Linear),
            )
            .unwrap(),
            pictogram_size: 0.33,
        }))
    }
}

impl SceneRenderPass for OverlayRenderPass {
    fn on_hdr_render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();
        let view_projection = ctx.camera.view_projection_matrix();
        let inv_view = ctx.camera.inv_view_matrix().unwrap();
        let camera_up = -inv_view.up();
        let camera_side = inv_view.side();
        let sound_icon = ctx
            .texture_cache
            .get(ctx.server, &self.sound_icon)
            .cloned()
            .unwrap();
        let light_icon = ctx.texture_cache.get(ctx.server, &self.light_icon).unwrap();

        for node in ctx.scene.graph.linear_iter() {
            let icon =
                if node.is_directional_light() || node.is_spot_light() || node.is_point_light() {
                    light_icon.clone()
                } else if node.is_sound() {
                    sound_icon.clone()
                } else {
                    continue;
                };

            let position = node.global_position();
            let world_matrix = Matrix4::new_translation(&position);

            let properties = PropertyGroup::from([
                property("viewProjectionMatrix", &view_projection),
                property("worldMatrix", &world_matrix),
                property("cameraSideVector", &camera_side),
                property("cameraUpVector", &camera_up),
                property("size", &self.pictogram_size),
            ]);
            let material = RenderMaterial::from([
                binding("diffuseTexture", (&icon.gpu_texture, &icon.gpu_sampler)),
                binding("properties", &properties),
            ]);

            stats += self.shader.run_pass(
                1,
                &ImmutableString::new("Primary"),
                ctx.framebuffer,
                &self.quad,
                ctx.viewport,
                &material,
                ctx.uniform_buffer_cache,
                Default::default(),
                None,
            )?;
        }

        Ok(stats)
    }

    fn source_type_id(&self) -> TypeId {
        TypeId::of::<Editor>()
    }
}
