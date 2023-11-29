use crate::{
    core::{
        log::{Log, MessageKind},
        scope_profile,
        sparse::SparseBuffer,
        sstorage::ImmutableString,
    },
    material::shader::{Shader, ShaderResource},
    renderer::{
        cache::CacheEntry,
        framework::{framebuffer::DrawParameters, gpu_program::GpuProgram, state::PipelineState},
    },
};
use fxhash::FxHashMap;
use fyrox_resource::entry::DEFAULT_RESOURCE_LIFETIME;

pub struct RenderPassData {
    pub program: GpuProgram,
    pub draw_params: DrawParameters,
}

pub struct ShaderSet {
    pub render_passes: FxHashMap<ImmutableString, RenderPassData>,
}

impl ShaderSet {
    pub fn new(state: &mut PipelineState, shader: &Shader) -> Option<Self> {
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
                    Log::writeln(
                        MessageKind::Error,
                        format!(
                            "Failed to create {} shader' GPU program. Reason: {:?}",
                            program_name, e
                        ),
                    );
                    return None;
                }
            };
        }

        Some(Self { render_passes: map })
    }
}

#[derive(Default)]
pub struct ShaderCache {
    pub(super) buffer: SparseBuffer<CacheEntry<ShaderSet>>,
}

impl ShaderCache {
    pub fn remove(&mut self, shader: &ShaderResource) {
        let mut state = shader.state();
        if let Some(shader_state) = state.data() {
            self.buffer.free(&shader_state.cache_index);
        }
    }

    pub fn get(
        &mut self,
        pipeline_state: &mut PipelineState,
        shader: &ShaderResource,
    ) -> Option<&ShaderSet> {
        scope_profile!();

        let key = shader.key();
        let mut shader_state = shader.state();

        if let Some(shader_state) = shader_state.data() {
            if self.buffer.is_index_valid(&shader_state.cache_index) {
                let entry = self.buffer.get_mut(&shader_state.cache_index).unwrap();

                // ShaderSet won't be destroyed while it used.
                entry.time_to_live = DEFAULT_RESOURCE_LIFETIME;

                Some(&entry.value)
            } else {
                let index = self.buffer.spawn(CacheEntry {
                    value: ShaderSet::new(pipeline_state, shader_state)?,
                    time_to_live: DEFAULT_RESOURCE_LIFETIME,
                    value_hash: key as u64,
                });
                shader_state.cache_index.set(index.get());
                Some(&self.buffer.get(&index).unwrap().value)
            }
        } else {
            None
        }
    }

    pub fn update(&mut self, dt: f32) {
        scope_profile!();

        for entry in self.buffer.iter_mut() {
            entry.time_to_live -= dt;
        }

        for i in 0..self.buffer.len() {
            if let Some(entry) = self.buffer.get_raw(i) {
                if entry.time_to_live <= 0.0 {
                    self.buffer.free_raw(i);
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
