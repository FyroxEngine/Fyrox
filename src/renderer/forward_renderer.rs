//! Forward renderer is used to render transparent meshes and meshes with custom blending options.
//!
//! # Notes
//!
//! This renderer eventually will replace deferred renderer, because deferred renderer is too restrictive.
//! For now it is used **only** to render transparent meshes (or any other mesh that has Forward render
//! path).

use crate::{
    core::{math::Rect, scope_profile, sstorage::ImmutableString},
    renderer::{
        apply_material,
        batch::RenderDataBatchStorage,
        cache::{shader::ShaderCache, texture::TextureCache},
        framework::{framebuffer::FrameBuffer, gpu_texture::GpuTexture, state::PipelineState},
        storage::MatrixStorage,
        GeometryCache, MaterialContext, QualitySettings, RenderPassStatistics,
    },
    scene::{camera::Camera, mesh::RenderPath},
};
use std::{cell::RefCell, rc::Rc};

pub(crate) struct ForwardRenderer {
    render_pass_name: ImmutableString,
}

pub(crate) struct ForwardRenderContext<'a, 'b> {
    pub state: &'a mut PipelineState,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub texture_cache: &'a mut TextureCache,
    pub shader_cache: &'a mut ShaderCache,
    pub batch_storage: &'a RenderDataBatchStorage,
    pub framebuffer: &'a mut FrameBuffer,
    pub viewport: Rect<i32>,
    pub quality_settings: &'a QualitySettings,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub black_dummy: Rc<RefCell<GpuTexture>>,
    pub volume_dummy: Rc<RefCell<GpuTexture>>,
    pub matrix_storage: &'a mut MatrixStorage,
}

impl ForwardRenderer {
    pub(crate) fn new() -> Self {
        Self {
            render_pass_name: ImmutableString::new("Forward"),
        }
    }

    pub(crate) fn render(&self, args: ForwardRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let ForwardRenderContext {
            state,
            camera,
            geom_cache,
            texture_cache,
            shader_cache,
            batch_storage,
            framebuffer,
            viewport,
            quality_settings,
            white_dummy,
            normal_dummy,
            black_dummy,
            volume_dummy,
            matrix_storage,
        } = args;

        let initial_view_projection = camera.view_projection_matrix();

        for batch in batch_storage
            .batches
            .iter()
            .filter(|b| b.render_path == RenderPath::Forward)
        {
            let material = batch.material.lock();
            let geometry = geom_cache.get(state, &batch.data);
            let blend_shapes_storage = batch
                .data
                .lock()
                .blend_shapes_container
                .as_ref()
                .and_then(|c| c.blend_shape_storage.clone());

            if let Some(render_pass) = shader_cache
                .get(state, material.shader())
                .and_then(|shader_set| shader_set.render_passes.get(&self.render_pass_name))
            {
                for instance in batch.instances.iter() {
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
                        |mut program_binding| {
                            apply_material(MaterialContext {
                                material: &material,
                                program_binding: &mut program_binding,
                                texture_cache,
                                world_matrix: &instance.world_transform,
                                wvp_matrix: &(view_projection * instance.world_transform),
                                bone_matrices: &instance.bone_matrices,
                                use_skeletal_animation: batch.is_skinned,
                                camera_position: &camera.global_position(),
                                use_pom: quality_settings.use_parallax_mapping,
                                light_position: &Default::default(),
                                blend_shapes_storage: blend_shapes_storage.as_ref(),
                                blend_shapes_weights: &instance.blend_shapes_weights,
                                normal_dummy: normal_dummy.clone(),
                                white_dummy: white_dummy.clone(),
                                black_dummy: black_dummy.clone(),
                                volume_dummy: volume_dummy.clone(),
                                matrix_storage,
                            });
                        },
                    );
                }
            }
        }

        statistics
    }
}
