use crate::resource::texture::Texture;
use crate::{
    core::{
        log::{Log, MessageKind},
        scope_profile,
    },
    renderer::{
        cache::TemporaryCache,
        framework::{
            error::FrameworkError,
            gpu_texture::{Coordinate, GpuTexture, PixelKind},
            state::PipelineState,
        },
    },
    resource::texture::TextureResource,
};
use std::{cell::RefCell, rc::Rc};

pub(crate) struct TextureRenderData {
    pub gpu_texture: Rc<RefCell<GpuTexture>>,
    pub modifications_counter: u64,
}

#[derive(Default)]
pub struct TextureCache {
    pub(crate) map: TemporaryCache<TextureRenderData>,
}

fn create_gpu_texture(
    state: &PipelineState,
    texture: &Texture,
) -> Result<TextureRenderData, FrameworkError> {
    GpuTexture::new(
        state,
        texture.kind().into(),
        PixelKind::from(texture.pixel_kind()),
        texture.minification_filter().into(),
        texture.magnification_filter().into(),
        texture.mip_count() as usize,
        Some(texture.data()),
    )
    .map(|gpu_texture| TextureRenderData {
        gpu_texture: Rc::new(RefCell::new(gpu_texture)),
        modifications_counter: texture.modifications_count(),
    })
}

impl TextureCache {
    /// Unconditionally uploads requested texture into GPU memory, previous GPU texture will be automatically
    /// destroyed.
    pub fn upload(
        &mut self,
        state: &PipelineState,
        texture: &TextureResource,
    ) -> Result<(), FrameworkError> {
        let mut texture = texture.state();
        if let Some(texture) = texture.data() {
            self.map.get_entry_mut_or_insert_with(
                &texture.cache_index,
                Default::default(),
                || create_gpu_texture(state, texture),
            )?;
            Ok(())
        } else {
            Err(FrameworkError::Custom(
                "Texture is not loaded yet!".to_string(),
            ))
        }
    }

    pub fn get(
        &mut self,
        state: &PipelineState,
        texture_resource: &TextureResource,
    ) -> Option<&Rc<RefCell<GpuTexture>>> {
        scope_profile!();

        let mut texture_data_guard = texture_resource.state();

        if let Some(texture) = texture_data_guard.data() {
            match self
                .map
                .get_mut_or_insert_with(&texture.cache_index, Default::default(), || {
                    create_gpu_texture(state, texture)
                }) {
                Ok(entry) => {
                    // Check if some value has changed in resource.

                    // Data might change from last frame, so we have to check it and upload new if so.
                    let modifications_count = texture.modifications_count();
                    if entry.modifications_counter != modifications_count {
                        let mut gpu_texture = entry.gpu_texture.borrow_mut();
                        if let Err(e) = gpu_texture.bind_mut(state, 0).set_data(
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
                            entry.modifications_counter = modifications_count;
                        }
                    }

                    let mut gpu_texture = entry.gpu_texture.borrow_mut();

                    let new_mag_filter = texture.magnification_filter().into();
                    if gpu_texture.magnification_filter() != new_mag_filter {
                        gpu_texture
                            .bind_mut(state, 0)
                            .set_magnification_filter(new_mag_filter);
                    }

                    let new_min_filter = texture.minification_filter().into();
                    if gpu_texture.minification_filter() != new_min_filter {
                        gpu_texture
                            .bind_mut(state, 0)
                            .set_minification_filter(new_min_filter);
                    }

                    if gpu_texture.anisotropy().ne(&texture.anisotropy_level()) {
                        gpu_texture
                            .bind_mut(state, 0)
                            .set_anisotropy(texture.anisotropy_level());
                    }

                    let new_s_wrap_mode = texture.s_wrap_mode().into();
                    if gpu_texture.s_wrap_mode() != new_s_wrap_mode {
                        gpu_texture
                            .bind_mut(state, 0)
                            .set_wrap(Coordinate::S, new_s_wrap_mode);
                    }

                    let new_t_wrap_mode = texture.t_wrap_mode().into();
                    if gpu_texture.t_wrap_mode() != new_t_wrap_mode {
                        gpu_texture
                            .bind_mut(state, 0)
                            .set_wrap(Coordinate::T, new_t_wrap_mode);
                    }

                    return Some(&entry.gpu_texture);
                }
                Err(e) => {
                    drop(texture_data_guard);
                    Log::writeln(
                        MessageKind::Error,
                        format!(
                            "Failed to create GPU texture from {} texture. Reason: {:?}",
                            texture_resource.kind(),
                            e
                        ),
                    );
                }
            }
        }
        None
    }

    pub fn update(&mut self, dt: f32) {
        self.map.update(dt)
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn unload(&mut self, texture: TextureResource) {
        if let Some(texture) = texture.state().data() {
            self.map.remove(&texture.cache_index);
        }
    }
}
