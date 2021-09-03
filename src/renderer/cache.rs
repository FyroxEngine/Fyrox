#![allow(missing_docs)] // TODO

use crate::renderer::framework::framebuffer::DrawParameters;
use crate::{
    asset::ResourceState,
    core::scope_profile,
    engine::resource_manager::DEFAULT_RESOURCE_LIFETIME,
    material::shader::{Shader, ShaderState},
    renderer::framework::{
        geometry_buffer::{
            BufferBuilder, ElementKind, GeometryBuffer, GeometryBufferBuilder, GeometryBufferKind,
        },
        gpu_program::GpuProgram,
        gpu_texture::{Coordinate, GpuTexture, PixelKind},
        state::PipelineState,
    },
    resource::texture::{Texture, TextureState},
    scene::mesh::surface::SurfaceData,
    utils::log::{Log, MessageKind},
};
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    ops::{Deref, DerefMut},
    rc::Rc,
};

pub struct CacheEntry<T> {
    pub value: T,
    pub value_hash: u64,
    pub time_to_live: f32,
}

impl<T> Deref for CacheEntry<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for CacheEntry<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

#[derive(Default)]
pub struct GeometryCache {
    map: HashMap<usize, CacheEntry<GeometryBuffer>>,
}

impl GeometryCache {
    pub fn get(&mut self, state: &mut PipelineState, data: &SurfaceData) -> &mut GeometryBuffer {
        scope_profile!();

        let key = (data as *const _) as usize;
        let data_hash = data.content_hash();

        let geometry_buffer = self.map.entry(key).or_insert_with(|| {
            let geometry_buffer = GeometryBufferBuilder::new(ElementKind::Triangle)
                .with_buffer_builder(BufferBuilder::from_vertex_buffer(
                    &data.vertex_buffer,
                    GeometryBufferKind::StaticDraw,
                ))
                .build(state)
                .unwrap();

            geometry_buffer
                .bind(state)
                .set_triangles(data.geometry_buffer.triangles_ref());

            CacheEntry {
                value: geometry_buffer,
                time_to_live: DEFAULT_RESOURCE_LIFETIME,
                value_hash: data_hash,
            }
        });

        if data_hash != geometry_buffer.value_hash {
            // Content has changed, upload new content.
            geometry_buffer.set_buffer_data(state, 0, data.vertex_buffer.raw_data());
            geometry_buffer
                .bind(state)
                .set_triangles(data.geometry_buffer.triangles_ref());

            geometry_buffer.value_hash = data_hash;
        }

        geometry_buffer.time_to_live = DEFAULT_RESOURCE_LIFETIME;
        geometry_buffer
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
}

#[derive(Default)]
pub struct TextureCache {
    pub(super) map: HashMap<usize, CacheEntry<Rc<RefCell<GpuTexture>>>>,
}

impl TextureCache {
    pub fn get(
        &mut self,
        state: &mut PipelineState,
        texture: &Texture,
    ) -> Option<Rc<RefCell<GpuTexture>>> {
        scope_profile!();

        let key = texture.key();
        let texture = texture.state();

        if let TextureState::Ok(texture) = texture.deref() {
            let entry = match self.map.entry(key) {
                Entry::Occupied(e) => {
                    let entry = e.into_mut();

                    // Texture won't be destroyed while it used.
                    entry.time_to_live = DEFAULT_RESOURCE_LIFETIME;

                    // Check if some value has changed in resource.

                    // Data might change from last frame, so we have to check it and upload new if so.
                    let data_hash = texture.data_hash();
                    if entry.value_hash != data_hash {
                        let mut tex = entry.borrow_mut();
                        if let Err(e) = tex.bind_mut(state, 0).set_data(
                            texture.kind().into(),
                            texture.pixel_kind().into(),
                            texture.mip_count() as usize,
                            Some(texture.data()),
                        ) {
                            Log::writeln(
                                MessageKind::Error,
                                format!(
                                    "Unable to upload new texture data to GPU. Reason: {:?}",
                                    e
                                ),
                            )
                        } else {
                            drop(tex);
                            // TODO: Is this correct to overwrite hash only if we've succeeded?
                            entry.value_hash = data_hash;
                        }
                    }

                    let mut tex = entry.borrow_mut();

                    let new_mag_filter = texture.magnification_filter().into();
                    if tex.magnification_filter() != new_mag_filter {
                        tex.bind_mut(state, 0)
                            .set_magnification_filter(new_mag_filter);
                    }

                    let new_min_filter = texture.minification_filter().into();
                    if tex.minification_filter() != new_min_filter {
                        tex.bind_mut(state, 0)
                            .set_minification_filter(new_min_filter);
                    }

                    if tex.anisotropy().ne(&texture.anisotropy_level()) {
                        tex.bind_mut(state, 0)
                            .set_anisotropy(texture.anisotropy_level());
                    }

                    let new_s_wrap_mode = texture.s_wrap_mode().into();
                    if tex.s_wrap_mode() != new_s_wrap_mode {
                        tex.bind_mut(state, 0)
                            .set_wrap(Coordinate::S, new_s_wrap_mode);
                    }

                    let new_t_wrap_mode = texture.t_wrap_mode().into();
                    if tex.t_wrap_mode() != new_t_wrap_mode {
                        tex.bind_mut(state, 0)
                            .set_wrap(Coordinate::T, new_t_wrap_mode);
                    }

                    std::mem::drop(tex);

                    entry
                }
                Entry::Vacant(e) => {
                    let gpu_texture = match GpuTexture::new(
                        state,
                        texture.kind().into(),
                        PixelKind::from(texture.pixel_kind()),
                        texture.minification_filter().into(),
                        texture.magnification_filter().into(),
                        texture.mip_count() as usize,
                        Some(texture.data()),
                    ) {
                        Ok(texture) => texture,
                        Err(e) => {
                            Log::writeln(
                                MessageKind::Error,
                                format!("Failed to create GPU texture. Reason: {:?}", e),
                            );
                            return None;
                        }
                    };

                    e.insert(CacheEntry {
                        value: Rc::new(RefCell::new(gpu_texture)),
                        time_to_live: DEFAULT_RESOURCE_LIFETIME,
                        value_hash: texture.data_hash(),
                    })
                }
            };

            Some(entry.value.clone())
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
            match GpuProgram::from_source(
                state,
                &render_pass.name,
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
                            render_pass.name, e
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
