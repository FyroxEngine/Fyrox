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
    asset::manager::ResourceManager,
    core::{
        algebra::{Matrix4, Point3, Vector3},
        color::Color,
        math::Rect,
    },
    graphics::{
        error::FrameworkError,
        framebuffer::{Attachment, GpuFrameBuffer},
        gpu_texture::{GpuTexture, GpuTextureDescriptor, GpuTextureKind, PixelKind},
        server::GraphicsServer,
    },
    renderer::{
        bundle::{BundleRenderContext, RenderDataBundleStorage, RenderDataBundleStorageOptions},
        cache::{
            shader::ShaderCache, texture::TextureCache, uniform::UniformMemoryAllocator,
            DynamicSurfaceCache,
        },
        observer::ObserverPosition,
        resources::RendererResources,
        settings::ShadowMapPrecision,
        shadow::cascade_size,
        utils::CubeMapFaceDescriptor,
        GeometryCache, RenderPassStatistics, POINT_SHADOW_PASS_NAME,
    },
    scene::{collider::BitMask, graph::Graph},
};

pub struct PointShadowMapRenderer {
    precision: ShadowMapPrecision,
    cascades: [GpuFrameBuffer; 3],
    size: usize,
    faces: [CubeMapFaceDescriptor; 6],
}

pub(crate) struct PointShadowMapRenderContext<'a> {
    pub render_mask: BitMask,
    pub elapsed_time: f32,
    pub server: &'a dyn GraphicsServer,
    pub graph: &'a Graph,
    pub light_pos: Vector3<f32>,
    pub light_radius: f32,
    pub geom_cache: &'a mut GeometryCache,
    pub cascade: usize,
    pub shader_cache: &'a mut ShaderCache,
    pub texture_cache: &'a mut TextureCache,
    pub renderer_resources: &'a RendererResources,
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,
    pub dynamic_surface_cache: &'a mut DynamicSurfaceCache,
    pub resource_manager: &'a ResourceManager,
}

impl PointShadowMapRenderer {
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
                "PointShadowMapDepthTexture",
                match precision {
                    ShadowMapPrecision::Full => PixelKind::D32F,
                    ShadowMapPrecision::Half => PixelKind::D16,
                },
                size,
                size,
            )?;

            let cube_map = server.create_texture(GpuTextureDescriptor {
                name: "PointLightShadowCubeMap",
                kind: GpuTextureKind::Cube { size },
                pixel_kind: PixelKind::R16F,
                ..Default::default()
            })?;

            server.create_frame_buffer(
                Some(Attachment::depth(depth)),
                vec![Attachment::color(cube_map)],
            )
        }

        Ok(Self {
            precision,
            cascades: [
                make_cascade(server, cascade_size(size, 0), precision)?,
                make_cascade(server, cascade_size(size, 1), precision)?,
                make_cascade(server, cascade_size(size, 2), precision)?,
            ],
            size,
            faces: CubeMapFaceDescriptor::cube_faces(),
        })
    }

    pub fn base_size(&self) -> usize {
        self.size
    }

    pub fn precision(&self) -> ShadowMapPrecision {
        self.precision
    }

    pub fn cascade_texture(&self, cascade: usize) -> &GpuTexture {
        &self.cascades[cascade].color_attachments()[0].texture
    }

    pub(crate) fn render(
        &mut self,
        args: PointShadowMapRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let _debug_scope = args.server.begin_scope("PointShadowMap");

        let mut statistics = RenderPassStatistics::default();

        let PointShadowMapRenderContext {
            elapsed_time,
            server,
            graph,
            render_mask,
            light_pos,
            light_radius,
            geom_cache,
            cascade,
            shader_cache,
            texture_cache,
            renderer_resources,
            uniform_memory_allocator,
            dynamic_surface_cache,
            resource_manager,
        } = args;

        let framebuffer = &self.cascades[cascade];
        let cascade_size = cascade_size(self.size, cascade);

        let viewport = Rect::new(0, 0, cascade_size as i32, cascade_size as i32);

        let z_near = 0.01;
        let z_far = light_radius;
        let light_projection_matrix =
            Matrix4::new_perspective(1.0, std::f32::consts::FRAC_PI_2, z_near, z_far);

        for face in self.faces.iter() {
            let _debug_scope = server.begin_scope(&format!("Face {:?}", face.face));

            framebuffer.set_cubemap_face(0, face.face, 0);
            framebuffer.clear(viewport, Some(Color::WHITE), Some(1.0), None);

            let light_look_at = light_pos + face.look;
            let light_view_matrix = Matrix4::look_at_rh(
                &Point3::from(light_pos),
                &Point3::from(light_look_at),
                &face.up,
            );

            let bundle_storage = RenderDataBundleStorage::from_graph(
                graph,
                render_mask,
                elapsed_time,
                &ObserverPosition {
                    translation: light_pos,
                    z_near,
                    z_far,
                    view_matrix: light_view_matrix,
                    projection_matrix: light_projection_matrix,
                    view_projection_matrix: light_projection_matrix * light_view_matrix,
                },
                POINT_SHADOW_PASS_NAME.clone(),
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
                    render_pass_name: &POINT_SHADOW_PASS_NAME,
                    frame_buffer: framebuffer,
                    viewport,
                    uniform_memory_allocator,
                    resource_manager,
                    use_pom: false,
                    light_position: &light_pos,
                    renderer_resources,
                    ambient_light: Color::WHITE, // TODO
                    scene_depth: None,
                },
            )?;
        }

        Ok(statistics)
    }
}
