//! Forward renderer is used to render transparent meshes and meshes with custom blending options.
//!
//! # Notes
//!  
//! This renderer eventually will replace deferred renderer, because deferred renderer is too restrictive.
//! For now it is used **only** to render transparent meshes (or any other mesh that has Forward render
//! path).

use crate::renderer::framework::gl;
use crate::{
    core::{math::Rect, scope_profile},
    renderer::{
        batch::BatchStorage,
        error::RendererError,
        framework::{
            framebuffer::{CullFace, DrawParameters, FrameBuffer, FrameBufferTrait},
            gpu_program::{GpuProgram, UniformLocation, UniformValue},
            state::PipelineState,
        },
        GeometryCache, RenderPassStatistics,
    },
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
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/forward_fs.glsl");
        let vertex_source = include_str!("shaders/forward_vs.glsl");
        let program = GpuProgram::from_source("ForwardShader", vertex_source, fragment_source)?;
        Ok(Self {
            wvp_matrix: program.uniform_location("worldViewProjection")?,
            diffuse_texture: program.uniform_location("diffuseTexture")?,
            color: program.uniform_location("color")?,
            use_skeletal_animation: program.uniform_location("useSkeletalAnimation")?,
            bone_matrices: program.uniform_location("boneMatrices")?,
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
    pub(in crate) fn new() -> Result<Self, RendererError> {
        Ok(Self {
            shader: Shader::new()?,
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

        state.set_blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

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
                        &[
                            (
                                self.shader.diffuse_texture,
                                UniformValue::Sampler {
                                    index: 0,
                                    texture: batch.diffuse_texture.clone(),
                                },
                            ),
                            (
                                self.shader.wvp_matrix,
                                UniformValue::Matrix4(view_projection * instance.world_transform),
                            ),
                            (
                                self.shader.use_skeletal_animation,
                                UniformValue::Bool(batch.is_skinned),
                            ),
                            (self.shader.color, UniformValue::Color(instance.color)),
                            (
                                self.shader.bone_matrices,
                                UniformValue::Mat4Array(instance.bone_matrices.as_slice()),
                            ),
                        ],
                    );
                }
            }
        }

        statistics
    }
}
