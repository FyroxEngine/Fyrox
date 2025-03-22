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

//! Generic, texture-based, storage for matrices with somewhat unlimited capacity.

use crate::{
    core::algebra::Matrix4,
    renderer::framework::{
        error::FrameworkError,
        gpu_texture::{GpuTextureDescriptor, GpuTextureKind, PixelKind},
        server::GraphicsServer,
    },
};
use fyrox_graphics::gpu_texture::GpuTexture;

/// Generic, texture-based, storage for matrices with somewhat unlimited capacity.
///
/// ## Motivation
///
/// Why it uses textures instead of SSBO? This could be done with SSBO, but it is not available on macOS because
/// SSBO was added only in OpenGL 4.3, but macOS support up to OpenGL 4.1.
pub struct MatrixStorage {
    texture: GpuTexture,
    matrices: Vec<Matrix4<f32>>,
}

impl MatrixStorage {
    /// Creates a new matrix storage.
    pub fn new(server: &dyn GraphicsServer) -> Result<Self, FrameworkError> {
        let identity = [Matrix4::<f32>::identity()];
        Ok(Self {
            texture: server.create_texture(GpuTextureDescriptor {
                kind: GpuTextureKind::Rectangle {
                    width: 4,
                    height: 1,
                },
                pixel_kind: PixelKind::RGBA32F,
                data: Some(crate::core::array_as_u8_slice(&identity)),
                ..Default::default()
            })?,
            matrices: Default::default(),
        })
    }

    /// Returns matrix storage texture.
    pub fn texture(&self) -> &GpuTexture {
        &self.texture
    }

    /// Updates contents of the internal texture with provided matrices.
    pub fn upload(
        &mut self,
        matrices: impl Iterator<Item = Matrix4<f32>>,
    ) -> Result<(), FrameworkError> {
        self.matrices.clear();
        self.matrices.extend(matrices);

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
            self.texture.set_data(
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
