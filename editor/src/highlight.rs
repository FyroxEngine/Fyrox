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
        core::{color::Color, pool::Handle, sstorage::ImmutableString},
        fxhash::FxHashSet,
        graph::{BaseSceneGraph, SceneGraph},
        renderer::{
            bundle::{BundleRenderContext, ObserverInfo, RenderContext, RenderDataBundleStorage},
            cache::shader::{
                binding, property, PropertyGroup, RenderMaterial, RenderPassContainer,
            },
            framework::{
                buffer::BufferUsage,
                error::FrameworkError,
                framebuffer::{Attachment, AttachmentKind, GpuFrameBuffer},
                geometry_buffer::GpuGeometryBuffer,
                gpu_texture::PixelKind,
                server::GraphicsServer,
                GeometryBufferExt,
            },
            make_viewport_matrix, RenderPassStatistics, SceneRenderPass, SceneRenderPassContext,
        },
        scene::{mesh::surface::SurfaceData, node::Node, Scene},
    },
    Editor,
};
use std::{any::TypeId, cell::RefCell, rc::Rc};

pub struct HighlightRenderPass {
    framebuffer: GpuFrameBuffer,
    quad: GpuGeometryBuffer,
    edge_detect_shader: RenderPassContainer,
    pub scene_handle: Handle<Scene>,
    pub nodes_to_highlight: FxHashSet<Handle<Node>>,
}

impl HighlightRenderPass {
    fn create_frame_buffer(
        server: &dyn GraphicsServer,
        mut width: usize,
        mut height: usize,
    ) -> GpuFrameBuffer {
        width = width.max(1);
        height = height.max(1);

        let depth_stencil = server
            .create_2d_render_target(PixelKind::D24S8, width, height)
            .unwrap();

        let frame_texture = server
            .create_2d_render_target(PixelKind::RGBA8, width, height)
            .unwrap();

        server
            .create_frame_buffer(
                Some(Attachment {
                    kind: AttachmentKind::DepthStencil,
                    texture: depth_stencil,
                }),
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: frame_texture,
                }],
            )
            .unwrap()
    }

    pub fn new_raw(server: &dyn GraphicsServer, width: usize, height: usize) -> Self {
        Self {
            framebuffer: Self::create_frame_buffer(server, width, height),
            quad: GpuGeometryBuffer::from_surface_data(
                &SurfaceData::make_unit_xy_quad(),
                BufferUsage::StaticDraw,
                server,
            )
            .unwrap(),
            edge_detect_shader: RenderPassContainer::from_str(
                server,
                include_str!("../resources/shaders/highlight.shader"),
            )
            .unwrap(),
            scene_handle: Default::default(),
            nodes_to_highlight: Default::default(),
        }
    }

    pub fn new(server: &dyn GraphicsServer, width: usize, height: usize) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self::new_raw(server, width, height)))
    }

    pub fn resize(&mut self, server: &dyn GraphicsServer, width: usize, height: usize) {
        self.framebuffer = Self::create_frame_buffer(server, width, height);
    }
}

impl SceneRenderPass for HighlightRenderPass {
    fn on_ldr_render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        if self.scene_handle != ctx.scene_handle {
            return Ok(Default::default());
        }

        // Draw selected nodes in the temporary frame buffer first.
        {
            let render_pass_name = ImmutableString::new("Forward");

            let observer_info = ObserverInfo {
                observer_position: ctx.camera.global_position(),
                z_near: ctx.camera.projection().z_near(),
                z_far: ctx.camera.projection().z_far(),
                view_matrix: ctx.camera.view_matrix(),
                projection_matrix: ctx.camera.projection_matrix(),
            };

            let mut render_bundle_storage =
                RenderDataBundleStorage::new_empty(observer_info.clone());

            let frustum = ctx.camera.frustum();
            let mut render_context = RenderContext {
                elapsed_time: ctx.elapsed_time,
                observer_info: &observer_info,
                frustum: Some(&frustum),
                storage: &mut render_bundle_storage,
                graph: &ctx.scene.graph,
                render_pass_name: &render_pass_name,
                dynamic_surface_cache: ctx.dynamic_surface_cache,
            };

            for &root_node_handle in self.nodes_to_highlight.iter() {
                if ctx.scene.graph.is_valid_handle(root_node_handle) {
                    for (_, node) in ctx.scene.graph.traverse_iter(root_node_handle) {
                        node.collect_render_data(&mut render_context);
                    }
                }
            }

            render_bundle_storage.sort();

            self.framebuffer
                .clear(ctx.viewport, Some(Color::TRANSPARENT), Some(1.0), None);

            stats += render_bundle_storage.render_to_frame_buffer(
                ctx.server,
                ctx.geometry_cache,
                ctx.shader_cache,
                |_| true,
                |_| true,
                BundleRenderContext {
                    texture_cache: ctx.texture_cache,
                    render_pass_name: &render_pass_name,
                    frame_buffer: &self.framebuffer,
                    use_pom: false,
                    light_position: &Default::default(),
                    fallback_resources: ctx.fallback_resources,
                    ambient_light: Default::default(),
                    scene_depth: Some(ctx.depth_texture),
                    viewport: ctx.viewport,
                    uniform_memory_allocator: ctx.uniform_memory_allocator,
                },
            )?;
        }

        // Render full screen quad with edge detect shader to draw outline of selected objects.
        {
            let frame_matrix = make_viewport_matrix(ctx.viewport);
            let frame_texture = &self.framebuffer.color_attachments()[0].texture;

            let color = Color::ORANGE.as_frgba();
            let properties = PropertyGroup::from([
                property("worldViewProjection", &frame_matrix),
                property("color", &color),
            ]);
            let material = RenderMaterial::from([
                binding(
                    "frameTexture",
                    (frame_texture, &ctx.fallback_resources.nearest_clamp_sampler),
                ),
                binding("properties", &properties),
            ]);

            stats += self.edge_detect_shader.run_pass(
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

        Ok(Default::default())
    }

    fn source_type_id(&self) -> TypeId {
        TypeId::of::<Editor>()
    }
}
