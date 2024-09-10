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
//!
//! # Notes
//!
//! This renderer eventually will replace deferred renderer, because deferred renderer is too restrictive.
//! For now it is used **only** to render transparent meshes (or any other mesh that has Forward render
//! path).

use crate::renderer::bundle::BundleRenderContext;
use crate::{
    core::{
        algebra::{Vector2, Vector4},
        color::Color,
        math::{frustum::Frustum, Matrix4Ext, Rect},
        scope_profile,
        sstorage::ImmutableString,
    },
    renderer::{
        bundle::RenderDataBundleStorage,
        cache::{shader::ShaderCache, texture::TextureCache},
        framework::{
            error::FrameworkError, framebuffer::FrameBuffer, gpu_texture::GpuTexture,
            state::PipelineState,
        },
        storage::MatrixStorageCache,
        GeometryCache, LightData, QualitySettings, RenderPassStatistics,
    },
    scene::{
        camera::Camera,
        graph::Graph,
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::RenderPath,
    },
};
use std::{cell::RefCell, rc::Rc};

pub(crate) struct ForwardRenderer {
    render_pass_name: ImmutableString,
}

pub(crate) struct ForwardRenderContext<'a, 'b> {
    pub state: &'a PipelineState,
    pub graph: &'b Graph,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub texture_cache: &'a mut TextureCache,
    pub shader_cache: &'a mut ShaderCache,
    pub bundle_storage: &'a RenderDataBundleStorage,
    pub framebuffer: &'a mut FrameBuffer,
    pub viewport: Rect<i32>,
    pub quality_settings: &'a QualitySettings,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub black_dummy: Rc<RefCell<GpuTexture>>,
    pub volume_dummy: Rc<RefCell<GpuTexture>>,
    pub scene_depth: Rc<RefCell<GpuTexture>>,
    pub matrix_storage: &'a mut MatrixStorageCache,
    pub ambient_light: Color,
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
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let ForwardRenderContext {
            state,
            graph,
            camera,
            geom_cache,
            texture_cache,
            shader_cache,
            bundle_storage,
            framebuffer,
            viewport,
            quality_settings,
            white_dummy,
            normal_dummy,
            black_dummy,
            volume_dummy,
            scene_depth,
            matrix_storage,
            ambient_light,
        } = args;

        let view_projection = camera.view_projection_matrix();

        let frustum = Frustum::from_view_projection_matrix(camera.view_projection_matrix())
            .unwrap_or_default();

        let inv_view = camera.inv_view_matrix().unwrap();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        let mut light_data = LightData::default();
        for light in graph.linear_iter() {
            if !light.global_visibility() || light_data.count == light_data.parameters.len() {
                continue;
            }

            let (radius, half_cone_angle_cos, half_hotspot_angle_cos, color) =
                if let Some(point) = light.cast::<PointLight>() {
                    (
                        point.radius(),
                        std::f32::consts::PI.cos(),
                        std::f32::consts::PI.cos(),
                        point.base_light_ref().color().as_frgb(),
                    )
                } else if let Some(spot) = light.cast::<SpotLight>() {
                    (
                        spot.distance(),
                        (spot.hotspot_cone_angle() * 0.5).cos(),
                        (spot.full_cone_angle() * 0.5).cos(),
                        spot.base_light_ref().color().as_frgb(),
                    )
                } else if let Some(directional) = light.cast::<DirectionalLight>() {
                    (
                        f32::INFINITY,
                        std::f32::consts::PI.cos(),
                        std::f32::consts::PI.cos(),
                        directional.base_light_ref().color().as_frgb(),
                    )
                } else {
                    continue;
                };

            if frustum.is_intersects_aabb(&light.world_bounding_box()) {
                let light_num = light_data.count;

                light_data.position[light_num] = light.global_position();
                light_data.direction[light_num] = light.up_vector();
                light_data.color_radius[light_num] =
                    Vector4::new(color.x, color.y, color.z, radius);
                light_data.parameters[light_num] =
                    Vector2::new(half_cone_angle_cos, half_hotspot_angle_cos);

                light_data.count += 1;
            }
        }

        for bundle in bundle_storage
            .bundles
            .iter()
            .filter(|b| b.render_path == RenderPath::Forward)
        {
            statistics += bundle.render_to_frame_buffer(
                state,
                geom_cache,
                shader_cache,
                |_| true,
                BundleRenderContext {
                    texture_cache,
                    render_pass_name: &self.render_pass_name,
                    frame_buffer: framebuffer,
                    viewport,
                    matrix_storage,
                    view_projection_matrix: &view_projection,
                    camera_position: &camera.global_position(),
                    camera_up_vector: &camera_up,
                    camera_side_vector: &camera_side,
                    z_near: camera.projection().z_near(),
                    z_far: camera.projection().z_far(),
                    use_pom: quality_settings.use_parallax_mapping,
                    light_position: &Default::default(),
                    normal_dummy: &normal_dummy,
                    white_dummy: &white_dummy,
                    black_dummy: &black_dummy,
                    volume_dummy: &volume_dummy,
                    light_data: Some(&light_data),
                    ambient_light,
                    scene_depth: Some(&scene_depth),
                },
            )?;
        }

        Ok(statistics)
    }
}
