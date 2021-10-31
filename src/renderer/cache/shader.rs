use crate::core::sstorage::ImmutableString;
use crate::{
    asset::ResourceState,
    core::{scope_profile, sparse::SparseBuffer},
    engine::resource_manager::DEFAULT_RESOURCE_LIFETIME,
    material::shader::{Shader, ShaderState},
    renderer::{
        cache::CacheEntry,
        framework::{framebuffer::DrawParameters, gpu_program::GpuProgram, state::PipelineState},
    },
    utils::log::{Log, MessageKind},
};
use fxhash::FxHashMap;
use std::ops::Deref;

pub struct RenderPassData {
    pub program: GpuProgram,
    pub draw_params: DrawParameters,
}

pub struct ShaderSet {
    pub render_passes: FxHashMap<ImmutableString, RenderPassData>,
}

impl ShaderSet {
    pub fn new(state: &mut PipelineState, shader: &ShaderState) -> Option<Self> {
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
    pub fn get(&mut self, state: &mut PipelineState, shader: &Shader) -> Option<&ShaderSet> {
        scope_profile!();

        let key = shader.key();
        let shader = shader.state();

        if let ResourceState::Ok(shader_state) = shader.deref() {
            if self.buffer.is_index_valid(&shader_state.cache_index) {
                Some(&self.buffer.get(&shader_state.cache_index).unwrap().value)
            } else {
                let index = self.buffer.spawn(CacheEntry {
                    value: ShaderSet::new(state, shader_state)?,
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
