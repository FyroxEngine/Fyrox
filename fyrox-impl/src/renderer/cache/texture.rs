// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    core::log::{Log, MessageKind},
    renderer::{
        cache::{TemporaryCache, TimeToLive},
        framework::{
            error::FrameworkError,
            gpu_texture::{Coordinate, GpuTexture, PixelKind},
            server::GraphicsServer,
        },
    },
    resource::texture::{Texture, TextureResource},
};
use std::{cell::RefCell, rc::Rc};

pub(crate) struct TextureRenderData {
    pub gpu_texture: Rc<RefCell<dyn GpuTexture>>,
    pub modifications_counter: u64,
}

#[derive(Default)]
pub struct TextureCache {
    cache: TemporaryCache<TextureRenderData>,
}

fn create_gpu_texture(
    server: &dyn GraphicsServer,
    texture: &Texture,
) -> Result<TextureRenderData, FrameworkError> {
    server
        .create_texture(
            texture.kind().into(),
            PixelKind::from(texture.pixel_kind()),
            texture.minification_filter().into(),
            texture.magnification_filter().into(),
            texture.mip_count() as usize,
            Some(texture.data()),
        )
        .map(|gpu_texture| TextureRenderData {
            gpu_texture,
            modifications_counter: texture.modifications_count(),
        })
}

impl TextureCache {
    /// Unconditionally uploads requested texture into GPU memory, previous GPU texture will be automatically
    /// destroyed.
    pub fn upload(
        &mut self,
        server: &dyn GraphicsServer,
        texture: &TextureResource,
    ) -> Result<(), FrameworkError> {
        let mut texture = texture.state();
        if let Some(texture) = texture.data() {
            self.cache.get_entry_mut_or_insert_with(
                &texture.cache_index,
                Default::default(),
                || create_gpu_texture(server, texture),
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
        server: &dyn GraphicsServer,
        texture_resource: &TextureResource,
    ) -> Option<&Rc<RefCell<dyn GpuTexture>>> {
        let mut texture_data_guard = texture_resource.state();

        if let Some(texture) = texture_data_guard.data() {
            match self.cache.get_mut_or_insert_with(
                &texture.cache_index,
                Default::default(),
                || create_gpu_texture(server, texture),
            ) {
                Ok(entry) => {
                    // Check if some value has changed in resource.

                    // Data might change from last frame, so we have to check it and upload new if so.
                    let modifications_count = texture.modifications_count();
                    if entry.modifications_counter != modifications_count {
                        let mut gpu_texture = entry.gpu_texture.borrow_mut();
                        if let Err(e) = gpu_texture.set_data(
                            texture.kind().into(),
                            texture.pixel_kind().into(),
                            texture.mip_count() as usize,
                            Some(texture.data()),
                        ) {
                            Log::writeln(
                                MessageKind::Error,
                                format!(
                                    "Unable to upload new texture data to GPU. Reason: {e:?}"
                                ),
                            )
                        } else {
                            entry.modifications_counter = modifications_count;
                        }
                    }

                    let mut gpu_texture = entry.gpu_texture.borrow_mut();

                    let new_mag_filter = texture.magnification_filter().into();
                    if gpu_texture.magnification_filter() != new_mag_filter {
                        gpu_texture.set_magnification_filter(new_mag_filter);
                    }

                    let new_min_filter = texture.minification_filter().into();
                    if gpu_texture.minification_filter() != new_min_filter {
                        gpu_texture.set_minification_filter(new_min_filter);
                    }

                    if gpu_texture.anisotropy().ne(&texture.anisotropy_level()) {
                        gpu_texture.set_anisotropy(texture.anisotropy_level());
                    }

                    let new_s_wrap_mode = texture.s_wrap_mode().into();
                    if gpu_texture.s_wrap_mode() != new_s_wrap_mode {
                        gpu_texture.set_wrap(Coordinate::S, new_s_wrap_mode);
                    }

                    let new_t_wrap_mode = texture.t_wrap_mode().into();
                    if gpu_texture.t_wrap_mode() != new_t_wrap_mode {
                        gpu_texture.set_wrap(Coordinate::T, new_t_wrap_mode);
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
        self.cache.update(dt)
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn unload(&mut self, texture: TextureResource) {
        if let Some(texture) = texture.state().data() {
            self.cache.remove(&texture.cache_index);
        }
    }

    pub fn alive_count(&self) -> usize {
        self.cache.alive_count()
    }

    /// Tries to bind existing GPU texture with a texture resource. If there's no such binding, then
    /// a new binding is created, otherwise - only the TTL is updated to keep the GPU texture alive
    /// for a certain time period (see [`TimeToLive`]).
    pub fn try_register(
        &mut self,
        texture: &TextureResource,
        gpu_texture: Rc<RefCell<dyn GpuTexture>>,
    ) {
        let data = texture.data_ref();
        let index = data.cache_index.clone();
        let entry = self.cache.get_mut(&index);
        if entry.is_none() {
            self.cache.spawn(
                TextureRenderData {
                    gpu_texture,
                    modifications_counter: data.modifications_count(),
                },
                index,
                TimeToLive::default(),
            );
        }
    }
}
