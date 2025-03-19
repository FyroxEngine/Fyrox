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
        algebra::{Matrix4, Point3, Vector2, Vector3},
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, frustum::Frustum, Rect},
    },
    renderer::{
        bundle::{
            BundleRenderContext, LightSource, LightSourceKind, ObserverInfo,
            RenderDataBundleStorage, RenderDataBundleStorageOptions,
        },
        cache::{
            geometry::GeometryCache, shader::ShaderCache, texture::TextureCache,
            uniform::UniformMemoryAllocator,
        },
        framework::{
            error::FrameworkError,
            framebuffer::{Attachment, AttachmentKind},
            gpu_texture::PixelKind,
            server::GraphicsServer,
        },
        FallbackResources, RenderPassStatistics, ShadowMapPrecision, DIRECTIONAL_SHADOW_PASS_NAME,
    },
    scene::{
        camera::Camera,
        graph::Graph,
        light::directional::{FrustumSplitOptions, CSM_NUM_CASCADES},
    },
};
use fyrox_graphics::framebuffer::GpuFrameBuffer;
use fyrox_graphics::gpu_texture::GpuTexture;

pub struct Cascade {
    pub frame_buffer: GpuFrameBuffer,
    pub view_proj_matrix: Matrix4<f32>,
    pub z_far: f32,
}

impl Cascade {
    pub fn new(
        server: &dyn GraphicsServer,
        size: usize,
        precision: ShadowMapPrecision,
    ) -> Result<Self, FrameworkError> {
        let depth = server.create_2d_render_target(
            match precision {
                ShadowMapPrecision::Full => PixelKind::D32F,
                ShadowMapPrecision::Half => PixelKind::D16,
            },
            size,
            size,
        )?;

        Ok(Self {
            frame_buffer: server.create_frame_buffer(
                Some(Attachment {
                    kind: AttachmentKind::Depth,
                    texture: depth,
                }),
                Default::default(),
            )?,
            view_proj_matrix: Default::default(),
            z_far: 0.0,
        })
    }

    pub fn texture(&self) -> &GpuTexture {
        &self.frame_buffer.depth_attachment().unwrap().texture
    }
}

pub struct CsmRenderer {
    cascades: [Cascade; CSM_NUM_CASCADES],
    size: usize,
    precision: ShadowMapPrecision,
}

pub(crate) struct CsmRenderContext<'a, 'c> {
    pub elapsed_time: f32,
    pub frame_size: Vector2<f32>,
    pub state: &'a dyn GraphicsServer,
    pub graph: &'c Graph,
    pub light: &'c LightSource,
    pub camera: &'c Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub shader_cache: &'a mut ShaderCache,
    pub texture_cache: &'a mut TextureCache,
    pub fallback_resources: &'a FallbackResources,
    pub uniform_memory_allocator: &'a mut UniformMemoryAllocator,
    pub dynamic_surface_cache: &'a mut DynamicSurfaceCache,
}

impl CsmRenderer {
    pub fn new(
        server: &dyn GraphicsServer,
        size: usize,
        precision: ShadowMapPrecision,
    ) -> Result<Self, FrameworkError> {
        Ok(Self {
            precision,
            size,
            cascades: [
                Cascade::new(server, size, precision)?,
                Cascade::new(server, size, precision)?,
                Cascade::new(server, size, precision)?,
            ],
        })
    }

    pub fn precision(&self) -> ShadowMapPrecision {
        self.precision
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn cascades(&self) -> &[Cascade] {
        &self.cascades
    }

    pub(crate) fn render(
        &mut self,
        ctx: CsmRenderContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let mut stats = RenderPassStatistics::default();

        let CsmRenderContext {
            elapsed_time,
            frame_size,
            state,
            graph,
            light,
            camera,
            geom_cache,
            shader_cache,
            texture_cache,
            fallback_resources,
            uniform_memory_allocator,
            dynamic_surface_cache,
        } = ctx;

        let LightSourceKind::Directional { ref csm_options } = light.kind else {
            return Ok(stats);
        };

        let light_direction = -light
            .up_vector
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(Vector3::y);

        let light_up_vec = light
            .look_vector
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(Vector3::z);

        let z_values = match csm_options.split_options {
            FrustumSplitOptions::Absolute { far_planes } => [
                camera.projection().z_near(),
                far_planes[0],
                far_planes[1],
                far_planes[2],
            ],
            FrustumSplitOptions::Relative { fractions } => [
                camera.projection().z_near(),
                camera.projection().z_far() * fractions[0],
                camera.projection().z_far() * fractions[1],
                camera.projection().z_far() * fractions[2],
            ],
        };

        for i in 0..CSM_NUM_CASCADES {
            let z_near = z_values[i];
            let mut z_far = z_values[i + 1];

            if z_far.eq(&z_near) {
                z_far += 10.0 * f32::EPSILON;
            }

            let projection_matrix = camera
                .projection()
                .clone()
                .with_z_near(z_near)
                .with_z_far(z_far)
                .matrix(frame_size);

            let frustum =
                Frustum::from_view_projection_matrix(projection_matrix * camera.view_matrix())
                    .unwrap_or_default();

            let center = frustum.center();
            let observer_position = center + light_direction;
            let light_view_matrix = Matrix4::look_at_lh(
                &Point3::from(observer_position),
                &Point3::from(center),
                &light_up_vec,
            );

            let mut aabb = AxisAlignedBoundingBox::default();
            for corner in frustum.corners() {
                let light_space_corner = light_view_matrix
                    .transform_point(&Point3::from(corner))
                    .coords;
                aabb.add_point(light_space_corner);
            }

            // Make sure most of the objects outside of the frustum will cast shadows.
            let z_mult = 10.0;
            if aabb.min.z < 0.0 {
                aabb.min.z *= z_mult;
            } else {
                aabb.min.z /= z_mult;
            }
            if aabb.max.z < 0.0 {
                aabb.max.z /= z_mult;
            } else {
                aabb.max.z *= z_mult;
            }

            let cascade_projection_matrix = Matrix4::new_orthographic(
                aabb.min.x, aabb.max.x, aabb.min.y, aabb.max.y, aabb.min.z, aabb.max.z,
            );

            let light_view_projection = cascade_projection_matrix * light_view_matrix;
            self.cascades[i].view_proj_matrix = light_view_projection;
            self.cascades[i].z_far = z_far;

            let viewport = Rect::new(0, 0, self.size as i32, self.size as i32);
            let framebuffer = &self.cascades[i].frame_buffer;
            framebuffer.clear(viewport, None, Some(1.0), None);

            let bundle_storage = RenderDataBundleStorage::from_graph(
                graph,
                elapsed_time,
                ObserverInfo {
                    observer_position,
                    z_near,
                    z_far,
                    view_matrix: light_view_matrix,
                    projection_matrix: cascade_projection_matrix,
                },
                DIRECTIONAL_SHADOW_PASS_NAME.clone(),
                RenderDataBundleStorageOptions {
                    collect_lights: false,
                },
                dynamic_surface_cache,
            );

            stats += bundle_storage.render_to_frame_buffer(
                state,
                geom_cache,
                shader_cache,
                |_| true,
                |_| true,
                BundleRenderContext {
                    texture_cache,
                    render_pass_name: &DIRECTIONAL_SHADOW_PASS_NAME,
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
        }

        Ok(stats)
    }
}
