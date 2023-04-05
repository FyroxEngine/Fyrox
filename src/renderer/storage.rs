//! Generic, texture-based, storage for matrices with somewhat unlimited capacity.

use crate::{
    core::algebra::Matrix4,
    renderer::framework::{
        error::FrameworkError,
        gpu_texture::{
            GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter, PixelKind,
        },
        state::PipelineState,
    },
    utils,
};
use std::{cell::RefCell, rc::Rc};

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
    pub fn new(state: &mut PipelineState) -> Result<Self, FrameworkError> {
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
                None,
            )?)),
            matrices: Default::default(),
        })
    }

    /// Returns matrix storage texture.
    pub fn texture(&self) -> &Rc<RefCell<GpuTexture>> {
        &self.texture
    }

    /// Updates contents of the internal texture with provided matrices.
    pub fn upload(
        &mut self,
        state: &mut PipelineState,
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
                    Some(utils::array_as_u8_slice(&self.matrices)),
                )?;
        }

        Ok(())
    }
}
