//! Forward renderer is used to render transparent meshes and meshes with custom blending options.
//!
//! # Notes
//!  
//! This renderer eventually will replace deferred renderer, because deferred renderer is too restrictive.
//! For now it is used **only** to render transparent meshes (or any other mesh that has Forward render
//! path).

use crate::{
    core::{math::Rect, scope_profile},
    renderer::framework::{
        error::FrameworkError,
        framebuffer::{CullFace, DrawParameters, FrameBuffer},
        gpu_program::{GpuProgram, UniformLocation},
        state::PipelineState,
    },
    renderer::{batch::BatchStorage, GeometryCache, RenderPassStatistics},
    scene::{camera::Camera, mesh::RenderPath},
};

pub struct Shader {
    program: GpuProgram,
    pub wvp_matrix: UniformLocation,
    pub diffuse_texture: UniformLocation,
    pub color: UniformLocation,
    pub use_skeletal_animation: UniformLocation,
    pub bone_matrices: UniformLocation,
}

impl Shader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("shaders/forward_fs.glsl");
        let vertex_source = include_str!("shaders/forward_vs.glsl");
        let program =
            GpuProgram::from_source(state, "ForwardShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location(state, "worldViewProjection")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            color: program.uniform_location(state, "color")?,
            use_skeletal_animation: program.uniform_location(state, "useSkeletalAnimation")?,
            bone_matrices: program.uniform_location(state, "boneMatrices")?,
            program,
        })
    }
}

pub(in crate) struct ForwardRenderer {
    shader: Shader,
}

pub(in crate) struct ForwardRenderContext<'a, 'b> {
    pub state: &'a mut PipelineState,
    pub camera: &'b Camera,
    pub geom_cache: &'a mut GeometryCache,
    pub batch_storage: &'a BatchStorage,
    pub framebuffer: &'a mut FrameBuffer,
    pub viewport: Rect<i32>,
}

impl ForwardRenderer {
    pub(in crate) fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            shader: Shader::new(state)?,
        })
    }

    pub(in crate) fn render(&self, args: ForwardRenderContext) -> RenderPassStatistics {
        scope_profile!();

        let mut statistics = RenderPassStatistics::default();

        let ForwardRenderContext {
            state,
            camera,
            geom_cache,
            batch_storage,
            framebuffer,
            viewport,
        } = args;

        let params = DrawParameters {
            cull_face: CullFace::Back,
            culling: true,
            color_write: Default::default(),
            depth_write: true,
            stencil_test: false,
            depth_test: true,
            blend: true, // TODO: Do not forget to change when renderer will have all features!
        };

        state.set_blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

        let initial_view_projection = camera.view_projection_matrix();

        for batch in batch_storage
            .batches
            .iter()
            .filter(|b| b.render_path == RenderPath::Forward)
        {
            let data = batch.data.read().unwrap();
            let geometry = geom_cache.get(state, &data);

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
                        &self.shader.program,
                        &params,
                        |program_binding| {
                            program_binding
                                .set_texture(&self.shader.diffuse_texture, &batch.diffuse_texture)
                                .set_matrix4(
                                    &self.shader.wvp_matrix,
                                    &(view_projection * instance.world_transform),
                                )
                                .set_bool(&self.shader.use_skeletal_animation, batch.is_skinned)
                                .set_color(&self.shader.color, &instance.color)
                                .set_matrix4_array(
                                    &self.shader.bone_matrices,
                                    instance.bone_matrices.as_slice(),
                                );
                        },
                    );
                }
            }
        }

        statistics
    }
}
