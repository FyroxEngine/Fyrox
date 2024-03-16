//! Generic, texture-based, storage for matrices with somewhat unlimited capacity.

use crate::{
    core::algebra::Matrix4,
    renderer::{
        bundle::PersistentIdentifier,
        framework::{
            error::FrameworkError,
            gpu_texture::{
                GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
            },
            state::PipelineState,
        },
    },
};
use fxhash::FxHashMap;
use std::{cell::RefCell, collections::hash_map::Entry, rc::Rc};

/// Generic, texture-based, storage for matrices with somewhat unlimited capacity.
///
/// ## Motivation
///
/// Why it uses textures instead of SSBO? This could be done with SSBO, but it is not available on macOS because
/// SSBO was added only in OpenGL 4.3, but macOS support up to OpenGL 4.1.
pub struct MatrixStorage {
    texture: Rc<RefCell<GpuTexture>>,
    matrices: Vec<Matrix4<f32>>,
}

impl MatrixStorage {
    /// Creates a new matrix storage.
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let identity = [Matrix4::<f32>::identity()];
        Ok(Self {
            texture: Rc::new(RefCell::new(GpuTexture::new(
                state,
                GpuTextureKind::Rectangle {
                    width: 4,
                    height: 1,
                },
                PixelKind::RGBA32F,
                MinificationFilter::Nearest,
                MagnificationFilter::Nearest,
                1,
                Some(crate::core::array_as_u8_slice(&identity)),
            )?)),
            matrices: Default::default(),
        })
    }

    /// Returns matrix storage texture.
    pub fn texture(&self) -> &Rc<RefCell<GpuTexture>> {
        &self.texture
    }

    /// Updates contents of the internal texture with provided matrices.
    fn upload(
        &mut self,
        state: &PipelineState,
        matrices: &[Matrix4<f32>],
        sampler: u32,
    ) -> Result<(), FrameworkError> {
        self.matrices.clear();
        self.matrices.extend_from_slice(matrices);

        // Select width for the texture by restricting width at 1024 pixels.
        let matrices_tex_size = 1024;
        let actual_matrices_pixel_count = self.matrices.len() * 4;
        let matrices_w = actual_matrices_pixel_count.min(matrices_tex_size);
        let matrices_h = (actual_matrices_pixel_count as f32 / matrices_w as f32)
            .ceil()
            .max(1.0) as usize;
        // Pad data to actual size.
        for _ in 0..(((matrices_w * matrices_h) - actual_matrices_pixel_count) / 4) {
            self.matrices.push(Default::default());
        }

        // Upload to GPU.
        if matrices_w != 0 && matrices_h != 0 {
            self.texture
                .borrow_mut()
                .bind_mut(state, sampler)
                .set_data(
                    GpuTextureKind::Rectangle {
                        width: matrices_w,
                        height: matrices_h,
                    },
                    PixelKind::RGBA32F,
                    1,
                    Some(crate::core::array_as_u8_slice(&self.matrices)),
                )?;
        }

        Ok(())
    }
}

/// A cache for matrix storages. It supplies the renderer with textures filled with matrices, usually
/// it is used to give unique storage for every entity that has bone matrices. Every storage in the
/// cache is re-used in the next frame.
pub struct MatrixStorageCache {
    empty: MatrixStorage,
    active_set: FxHashMap<PersistentIdentifier, MatrixStorage>,
    cache: Vec<MatrixStorage>,
}

impl MatrixStorageCache {
    /// Creates new cache.
    pub fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        Ok(Self {
            empty: MatrixStorage::new(state)?,
            active_set: Default::default(),
            cache: Default::default(),
        })
    }

    /// Clears active set of the cache and prepares the cache for the a new frame.
    pub fn begin_frame(&mut self) {
        for (_, storage) in self.active_set.drain() {
            self.cache.push(storage);
        }
    }

    /// Tries to upload the given set of matrices to a GPU matrix storage associated with some persistent
    /// identifier. Main idea of this method is to give every entity with a persistent id its own matrix
    /// storage which prevents implicit synchronization step in the video driver. Using a single texture
    /// and changing its content dozens of time per frame could be bad for performance, because of implicit
    /// synchronization.  
    pub fn try_bind_and_upload(
        &mut self,
        state: &PipelineState,
        id: PersistentIdentifier,
        matrices: &[Matrix4<f32>],
        sampler: u32,
    ) -> Result<&MatrixStorage, FrameworkError> {
        if matrices.is_empty() {
            // Bind empty storage if input matrices set is empty.
            self.empty.texture().borrow().bind(state, sampler);
            Ok(&self.empty)
        } else {
            // Otherwise, try to fetch a storage using persistent id and use it (or create new if there's
            // no vacant storage in the cache).
            match self.active_set.entry(id) {
                Entry::Occupied(existing) => {
                    existing.get().texture.borrow().bind(state, sampler);
                    Ok(existing.into_mut())
                }
                Entry::Vacant(entry) => {
                    let mut storage = if let Some(cached) = self.cache.pop() {
                        cached
                    } else {
                        MatrixStorage::new(state)?
                    };

                    storage.upload(state, matrices, sampler)?;

                    Ok(entry.insert(storage))
                }
            }
        }
    }
}
