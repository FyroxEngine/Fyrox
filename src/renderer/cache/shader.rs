use crate::renderer::cache::TemporaryCache;
use crate::renderer::framework::error::FrameworkError;
use crate::{
    core::sstorage::ImmutableString,
    material::shader::{Shader, ShaderResource},
    renderer::framework::{
        framebuffer::DrawParameters, gpu_program::GpuProgram, state::PipelineState,
    },
};
use fxhash::FxHashMap;
use fyrox_core::log::Log;

pub struct RenderPassData {
    pub program: GpuProgram,
    pub draw_params: DrawParameters,
}

pub struct ShaderSet {
    pub render_passes: FxHashMap<ImmutableString, RenderPassData>,
}

impl ShaderSet {
    pub fn new(state: &PipelineState, shader: &Shader) -> Result<Self, FrameworkError> {
        let mut map = FxHashMap::default();
        for render_pass in shader.definition.passes.iter() {
            let program_name = format!("{}_{}", shader.definition.name, render_pass.name);
            match GpuProgram::from_source(
                state,
                &program_name,
                &render_pass.vertex_shader,
                &render_pass.fragment_shader,
            ) {
                Ok(gpu_program) => {
                    map.insert(
                        ImmutableString::new(&render_pass.name),
                        RenderPassData {
                            program: gpu_program,
                            draw_params: render_pass.draw_parameters.clone(),
                        },
                    );
                }
                Err(e) => {
                    return Err(FrameworkError::Custom(format!(
                        "Failed to create {} shader' GPU program. Reason: {:?}",
                        program_name, e
                    )));
                }
            };
        }

        Ok(Self { render_passes: map })
    }
}

#[derive(Default)]
pub struct ShaderCache {
    pub(super) cache: TemporaryCache<ShaderSet>,
}

impl ShaderCache {
    pub fn remove(&mut self, shader: &ShaderResource) {
        let mut state = shader.state();
        if let Some(shader_state) = state.data() {
            self.cache.remove(&shader_state.cache_index);
        }
    }

    pub fn get(
        &mut self,
        pipeline_state: &PipelineState,
        shader: &ShaderResource,
    ) -> Option<&ShaderSet> {
        let mut shader_state = shader.state();

        if let Some(shader_state) = shader_state.data() {
            match self.cache.get_or_insert_with(
                &shader_state.cache_index,
                Default::default(),
                || ShaderSet::new(pipeline_state, shader_state),
            ) {
                Ok(shader_set) => Some(shader_set),
                Err(error) => {
                    Log::err(format!("{}", error));
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.cache.update(dt)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
