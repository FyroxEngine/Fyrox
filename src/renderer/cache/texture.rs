use crate::{
    asset::entry::DEFAULT_RESOURCE_LIFETIME,
    core::{
        log::{Log, MessageKind},
        scope_profile,
    },
    renderer::{
        cache::CacheEntry,
        framework::{
            error::FrameworkError,
            gpu_texture::{Coordinate, GpuTexture, PixelKind},
            state::PipelineState,
        },
    },
    resource::texture::TextureResource,
};
use fxhash::FxHashMap;
use std::{cell::RefCell, collections::hash_map::Entry, rc::Rc};

#[derive(Default)]
pub struct TextureCache {
    pub(crate) map: FxHashMap<usize, CacheEntry<Rc<RefCell<GpuTexture>>>>,
}

impl TextureCache {
    /// Unconditionally uploads requested texture into GPU memory, previous GPU texture will be automatically
    /// destroyed.
    pub fn upload(
        &mut self,
        state: &mut PipelineState,
        texture: &TextureResource,
    ) -> Result<(), FrameworkError> {
        let key = texture.key();
        let mut texture = texture.state();

        if let Some(texture) = texture.data() {
            let gpu_texture = GpuTexture::new(
                state,
                texture.kind().into(),
                PixelKind::from(texture.pixel_kind()),
                texture.minification_filter().into(),
                texture.magnification_filter().into(),
                texture.mip_count() as usize,
                Some(texture.data()),
            )?;

            match self.map.entry(key) {
                Entry::Occupied(mut e) => {
                    *e.get_mut().value.borrow_mut() = gpu_texture;
                }
                Entry::Vacant(e) => {
                    e.insert(CacheEntry {
                        value: Rc::new(RefCell::new(gpu_texture)),
                        time_to_live: DEFAULT_RESOURCE_LIFETIME,
                        value_hash: texture.data_hash(),
                    });
                }
            }

            Ok(())
        } else {
            Err(FrameworkError::Custom(
                "Texture is not loaded yet!".to_string(),
            ))
        }
    }

    pub fn get(
        &mut self,
        state: &mut PipelineState,
        texture_resource: &TextureResource,
    ) -> Option<Rc<RefCell<GpuTexture>>> {
        scope_profile!();

        let key = texture_resource.key();

        let mut texture_data_guard = texture_resource.state();

        if let Some(texture) = texture_data_guard.data() {
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
                            drop(texture_data_guard);

                            Log::writeln(
                                MessageKind::Error,
                                format!("Failed to create GPU texture from {} engine texture. Reason: {:?}", texture_resource.kind(), e),
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

    pub fn unload(&mut self, texture: TextureResource) {
        self.map.remove(&texture.key());
    }
}
