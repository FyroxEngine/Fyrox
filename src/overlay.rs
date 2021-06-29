use crate::load_image;
use rg3d::core::algebra::Matrix4;
use rg3d::core::math::Matrix4Ext;
use rg3d::resource::texture::{CompressionOptions, Texture};
use rg3d::sound::source::SoundSource;
use rg3d::{
    renderer::{
        framework::{
            error::FrameworkError,
            framebuffer::{CullFace, DrawParameters},
            gpu_program::{GpuProgram, UniformLocation},
            state::PipelineState,
        },
        RenderPassStatistics, SceneRenderPass, SceneRenderPassContext,
    },
    scene::mesh::surface::SurfaceData,
};
use std::sync::{Arc, Mutex};

struct OverlayShader {
    program: GpuProgram,
    view_projection_matrix: UniformLocation,
    world_matrix: UniformLocation,
    camera_side_vector: UniformLocation,
    camera_up_vector: UniformLocation,
    diffuse_texture: UniformLocation,
    size: UniformLocation,
}

impl OverlayShader {
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../resources/embed/shaders/overlay_fs.glsl");
        let vertex_source = include_str!("../resources/embed/shaders/overlay_vs.glsl");
        let program = GpuProgram::from_source(state, "FlatShader", vertex_source, fragment_source)?;
        Ok(Self {
            view_projection_matrix: program.uniform_location(state, "viewProjectionMatrix")?,
            world_matrix: program.uniform_location(state, "worldMatrix")?,
            camera_side_vector: program.uniform_location(state, "cameraSideVector")?,
            camera_up_vector: program.uniform_location(state, "cameraUpVector")?,
            size: program.uniform_location(state, "size")?,
            diffuse_texture: program.uniform_location(state, "diffuseTexture")?,
            program,
        })
    }
}

pub struct OverlayRenderPass {
    quad: SurfaceData,
    shader: OverlayShader,
    sound_icon: Texture,
}

impl OverlayRenderPass {
    pub fn new(state: &mut PipelineState) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            quad: SurfaceData::make_collapsed_xy_quad(),
            shader: OverlayShader::new(state).unwrap(),
            sound_icon: Texture::load_from_memory(
                include_bytes!("../resources/embed/sound_source.png"),
                CompressionOptions::NoCompression,
            )
            .unwrap(),
        }))
    }
}

impl SceneRenderPass for OverlayRenderPass {
    fn render(
        &mut self,
        ctx: SceneRenderPassContext,
    ) -> Result<RenderPassStatistics, FrameworkError> {
        let state = ctx.scene.sound_context.state();

        let view_projection = ctx.camera.view_projection_matrix();
        let shader = &self.shader;
        let inv_view = ctx.camera.inv_view_matrix().unwrap();
        let camera_up = inv_view.up();
        let camera_side = inv_view.side();
        let sound_icon = ctx
            .texture_cache
            .get(ctx.pipeline_state, &self.sound_icon)
            .unwrap();

        for source in state.sources().iter().filter_map(|s| {
            if let SoundSource::Spatial(spatial) = s {
                Some(spatial)
            } else {
                None
            }
        }) {
            let world_matrix = Matrix4::new_translation(&source.position());

            ctx.framebuffer.draw(
                ctx.geometry_cache.get(ctx.pipeline_state, &self.quad),
                ctx.pipeline_state,
                ctx.viewport,
                &shader.program,
                &DrawParameters {
                    cull_face: CullFace::Back,
                    culling: false,
                    color_write: Default::default(),
                    depth_write: false,
                    stencil_test: false,
                    depth_test: false,
                    blend: true,
                },
                |program_binding| {
                    program_binding
                        .set_matrix4(&shader.view_projection_matrix, &view_projection)
                        .set_matrix4(&shader.world_matrix, &world_matrix)
                        .set_vector3(&shader.camera_side_vector, &camera_side)
                        .set_vector3(&shader.camera_up_vector, &camera_up)
                        .set_float(&shader.size, 0.33)
                        .set_texture(&shader.diffuse_texture, &sound_icon);
                },
            );
        }

        Ok(Default::default())
    }
}
