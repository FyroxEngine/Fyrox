use crate::{
    asset::ResourceState,
    core::scope_profile,
    engine::resource_manager::DEFAULT_RESOURCE_LIFETIME,
    material::shader::{Shader, ShaderState},
    renderer::{
        cache::CacheEntry,
        framework::{framebuffer::DrawParameters, gpu_program::GpuProgram, state::PipelineState},
    },
    resource::texture::Texture,
    utils::log::{Log, MessageKind},
};
use std::{
    collections::{hash_map::Entry, HashMap},
    ops::Deref,
};

pub struct RenderPassData {
    pub program: GpuProgram,
    pub draw_params: DrawParameters,
}

pub struct ShaderSet {
    pub render_passes: HashMap<String, RenderPassData>,
}

impl ShaderSet {
    pub fn new(state: &mut PipelineState, shader: &ShaderState) -> Option<Self> {
        let mut map = HashMap::new();
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
                        render_pass.name.clone(),
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
    pub(super) map: HashMap<usize, CacheEntry<ShaderSet>>,
}

impl ShaderCache {
    pub fn get(&mut self, state: &mut PipelineState, shader: &Shader) -> Option<&ShaderSet> {
        scope_profile!();

        let key = shader.key();
        let shader = shader.state();

        if let ResourceState::Ok(shader_state) = shader.deref() {
            let entry = match self.map.entry(key) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => e.insert(CacheEntry {
                    value: ShaderSet::new(state, shader_state)?,
                    time_to_live: DEFAULT_RESOURCE_LIFETIME,
                    value_hash: key as u64,
                }),
            };

            Some(&entry.value)
        } else {
            None
        }
    }

    pub fn update(&mut self, dt: f32) {
        scope_profile!();

        for entry in self.map.values_mut() {
            entry.time_to_live -= dt;
        }

        self.map.retain(|_, v| v.time_to_live > 0.0);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn unload(&mut self, texture: Texture) {
        self.map.remove(&texture.key());
    }
}
