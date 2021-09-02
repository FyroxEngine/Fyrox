//! Forward renderer is used to render transparent meshes and meshes with custom blending options.
//!
//! # Notes
//!  
//! This renderer eventually will replace deferred renderer, because deferred renderer is too restrictive.
//! For now it is used **only** to render transparent meshes (or any other mesh that has Forward render
//! path).

use crate::renderer::MaterialContext;
use crate::{
    core::{math::Rect, scope_profile},
    renderer::{
        apply_material,
        batch::BatchStorage,
        cache::{ShaderCache, TextureCache},
        framework::{
            framebuffer::{CullFace, DrawParameters, FrameBuffer},
            gpu_texture::GpuTexture,
            state::PipelineState,
        },
        GeometryCache, QualitySettings, RenderPassStatistics,
    },
    scene::{camera::Camera, mesh::RenderPath},
};
use std::{cell::RefCell, rc::Rc};

pub(in crate) struct ForwardRenderer {}

pub(in crate) struct ForwardRenderContext<'a, 'b> {
    pub state: &'a mut PipelineState,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub texture_cache: &'a mut TextureCache,
    pub shader_cache: &'a mut ShaderCache,
    pub batch_storage: &'a BatchStorage,
    pub framebuffer: &'a mut FrameBuffer,
    pub viewport: Rect<i32>,
    pub quality_settings: &'a QualitySettings,
    pub white_dummy: Rc<RefCell<GpuTexture>>,
    pub normal_dummy: Rc<RefCell<GpuTexture>>,
    pub black_dummy: Rc<RefCell<GpuTexture>>,
}

impl ForwardRenderer {
    pub(in crate) fn new() -> Self {
        Self {}
    }

    pub(in crate) fn render(&self, args: ForwardRenderContext) -> RenderPassStatistics {
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
        } = args;

        let params = DrawParameters {
            cull_face: Some(CullFace::Back),
            color_write: Default::default(),
            depth_write: true,
            stencil_test: false,
            depth_test: true,
            blend: true,
        };

        state.set_blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

        let initial_view_projection = camera.view_projection_matrix();

        for batch in batch_storage
            .batches
            .iter()
            .filter(|b| b.render_path == RenderPath::Forward)
        {
            let material = batch.material.lock().unwrap();
            let data = batch.data.read().unwrap();
            let geometry = geom_cache.get(state, &data);

            if let Some(shader_set) = shader_cache.get(state, material.shader()) {
                if let Some(program) = shader_set.map.get("Forward") {
                    for instance in batch.instances.iter() {
                        if camera.visibility_cache.is_visible(instance.owner) {
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
                                program,
                                &params,
                                |mut program_binding| {
                                    apply_material(MaterialContext {
                                        material: &*material,
                                        program_binding: &mut program_binding,
                                        texture_cache,
                                        world_matrix: &instance.world_transform,
                                        wvp_matrix: &(view_projection * instance.world_transform),
                                        bone_matrices: &instance.bone_matrices,
                                        use_skeletal_animation: batch.is_skinned,
                                        camera_position: &camera.global_position(),
                                        use_pom: quality_settings.use_parallax_mapping,
                                        light_position: &Default::default(),
                                        normal_dummy: normal_dummy.clone(),
                                        white_dummy: white_dummy.clone(),
                                        black_dummy: black_dummy.clone(),
                                    });
                                },
                            );
                        }
                    }
                }
            }
        }

        statistics
    }
}
