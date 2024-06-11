//! Forward renderer is used to render transparent meshes and meshes with custom blending options.
//!
//! # Notes
//!
//! This renderer eventually will replace deferred renderer, because deferred renderer is too restrictive.
//! For now it is used **only** to render transparent meshes (or any other mesh that has Forward render
//! path).

use crate::{
    core::{
        algebra::{Vector2, Vector4},
        color::Color,
        math::{frustum::Frustum, Rect},
        scope_profile,
        sstorage::ImmutableString,
    },
    renderer::{
        apply_material,
        bundle::RenderDataBundleStorage,
        cache::{shader::ShaderCache, texture::TextureCache},
        framework::{
            error::FrameworkError, framebuffer::FrameBuffer, gpu_texture::GpuTexture,
            state::PipelineState,
        },
        storage::MatrixStorageCache,
        GeometryCache, LightData, MaterialContext, QualitySettings, RenderPassStatistics,
    },
    scene::{
        camera::Camera,
        graph::Graph,
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::RenderPath,
    },
};
use fyrox_core::math::Matrix4Ext;
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

        let initial_view_projection = camera.view_projection_matrix();

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
            let mut material_state = bundle.material.state();

            let Some(material) = material_state.data() else {
                continue;
            };

            let Some(geometry) = geom_cache.get(state, &bundle.data, bundle.time_to_live) else {
                continue;
            };

            let blend_shapes_storage = bundle
                .data
                .data_ref()
                .blend_shapes_container
                .as_ref()
                .and_then(|c| c.blend_shape_storage.clone());

            let Some(render_pass) = shader_cache
                .get(state, material.shader())
                .and_then(|shader_set| shader_set.render_passes.get(&self.render_pass_name))
            else {
                continue;
            };

            for instance in bundle.instances.iter() {
                let view_projection = if instance.depth_offset != 0.0 {
                    let mut projection = camera.projection_matrix();
                    projection[14] -= instance.depth_offset;
                    projection * camera.view_matrix()
                } else {
                    initial_view_projection
                };

                statistics += framebuffer.draw(
                    geometry,
                    state,
                    viewport,
                    &render_pass.program,
                    &render_pass.draw_params,
                    instance.element_range,
                    |mut program_binding| {
                        apply_material(MaterialContext {
                            material,
                            program_binding: &mut program_binding,
                            texture_cache,
                            world_matrix: &instance.world_transform,
                            view_projection_matrix: &view_projection,
                            wvp_matrix: &(view_projection * instance.world_transform),
                            bone_matrices: &instance.bone_matrices,
                            use_skeletal_animation: bundle.is_skinned,
                            camera_position: &camera.global_position(),
                            camera_up_vector: &camera_up,
                            camera_side_vector: &camera_side,
                            z_near: camera.projection().z_near(),
                            z_far: camera.projection().z_far(),
                            use_pom: quality_settings.use_parallax_mapping,
                            light_position: &Default::default(),
                            blend_shapes_storage: blend_shapes_storage.as_ref(),
                            blend_shapes_weights: &instance.blend_shapes_weights,
                            normal_dummy: &normal_dummy,
                            white_dummy: &white_dummy,
                            black_dummy: &black_dummy,
                            volume_dummy: &volume_dummy,
                            matrix_storage,
                            persistent_identifier: instance.persistent_identifier,
                            light_data: Some(&light_data),
                            ambient_light,
                            scene_depth: Some(&scene_depth),
                        });
                    },
                )?;
            }
        }

        Ok(statistics)
    }
}
