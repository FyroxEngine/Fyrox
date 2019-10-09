use crate::{
    renderer::{
        gl::types::GLuint,
        gl,
        error::RendererError,
    }
};
use std::{
    ffi::c_void,
    mem::size_of
};
use crate::resource::texture::TextureKind;

#[derive(Copy, Clone)]
pub enum GpuTextureKind {
    Line {
        length: usize,
    },
    Rectangle {
        width: usize,
        height: usize,
    },
    Cube {
        width: usize,
        height: usize,
    },
    Volume {
        width: usize,
        height: usize,
        depth: usize,
    },
}

impl GpuTextureKind {
    fn to_texture_target(&self) -> GLuint {
        match self {
            GpuTextureKind::Line { .. } => gl::TEXTURE_1D,
            GpuTextureKind::Rectangle { .. } => gl::TEXTURE_2D,
            GpuTextureKind::Cube { .. } => gl::TEXTURE_CUBE_MAP,
            GpuTextureKind::Volume { .. } => gl::TEXTURE_3D,
        }
    }
}

#[derive(Copy, Clone)]
pub enum PixelKind {
    RGBA8,
    RGB8,
    RG8,
    R8,
}

impl From<TextureKind> for PixelKind {
    fn from(texture_kind: TextureKind) -> Self {
        match texture_kind {
            TextureKind::R8 => PixelKind::R8,
            TextureKind::RGB8 => PixelKind::RGB8,
            TextureKind::RGBA8 => PixelKind::RGBA8,
        }
    }
}

pub struct GpuTexture {
    texture: GLuint,
    kind: GpuTextureKind
}

impl PixelKind {
    fn size_bytes(self) -> usize {
        match self {
            PixelKind::RGBA8 => 4 * size_of::<u8>(),
            PixelKind::RGB8 => 3 * size_of::<u8>(),
            PixelKind::RG8 => 2 * size_of::<u8>(),
            PixelKind::R8 => size_of::<u8>(),
        }
    }
}

impl GpuTexture {
    /// Creates new GPU texture of specified kind
    ///
    /// Notes: in case of Cube texture, `bytes` should contain all 6 cube faces ordered like so,
    /// +X, -X, +Y, -Y, +Z, -Z
    pub fn new(kind: GpuTextureKind,
               pixel_kind: PixelKind,
               bytes: &[u8],
               generate_mipmaps: bool) -> Result<Self, RendererError> {
        let bytes_per_pixel = pixel_kind.size_bytes();

        let desired_byte_count = match kind {
            GpuTextureKind::Line { length } => length * bytes_per_pixel,
            GpuTextureKind::Rectangle { width, height } => width * height * bytes_per_pixel,
            GpuTextureKind::Cube { width, height } => 6 * width * height * bytes_per_pixel,
            GpuTextureKind::Volume { width, height, depth } => {
                width * height * depth * bytes_per_pixel
            }
        };

        if bytes.len() != desired_byte_count {
            return Err(RendererError::InvalidTextureData);
        };

        let target = kind.to_texture_target();

        unsafe {
            let mut texture = 0;
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(target, texture);

            let (type_, format, internal_format) = match pixel_kind {
                PixelKind::RGBA8 => (gl::UNSIGNED_BYTE, gl::RGBA, gl::RGBA8),
                PixelKind::RGB8 => (gl::UNSIGNED_BYTE, gl::RGB, gl::RGB8),
                PixelKind::RG8 => (gl::UNSIGNED_BYTE, gl::RG, gl::RG8),
                PixelKind::R8 => (gl::UNSIGNED_BYTE, gl::RED, gl::R8),
            };

            match pixel_kind {
                PixelKind::RGBA8 | PixelKind::RGB8 => {
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4)
                },
                PixelKind::RG8 => {
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 2)
                },
                PixelKind::R8 => {
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1)
                },
            }

            let pixels = bytes.as_ptr() as *const c_void;

            match kind {
                GpuTextureKind::Line { length } => {
                    gl::TexImage1D(gl::TEXTURE_1D, 0, internal_format as i32,
                                   length as i32, 0, format, type_, pixels);
                }
                GpuTextureKind::Rectangle { width, height } => {
                    gl::TexImage2D(gl::TEXTURE_2D, 0, internal_format as i32,
                                   width as i32, height as i32, 0,
                                   format, type_, pixels);
                }
                GpuTextureKind::Cube { width, height } => {
                    for face in 0..6 {
                        let bytes_per_face = width * height * bytes_per_pixel;

                        let begin = face * bytes_per_face;
                        let end = (face + 1) * bytes_per_face;

                        let face_pixels = bytes[begin..end].as_ptr() as *const c_void;

                        gl::TexImage2D(gl::TEXTURE_CUBE_MAP_POSITIVE_X + face as u32, 0,
                                       internal_format as i32, width as i32,
                                       height as i32, 0, format, type_, face_pixels);
                    }
                }
                GpuTextureKind::Volume { width, height, depth } => {
                    gl::TexImage3D(gl::TEXTURE_3D, 0, internal_format as i32,
                                   width as i32, height as i32, depth as i32,
                                   0, format, type_, pixels);
                }
            }

            gl::TexParameteri(target, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            if generate_mipmaps {
                gl::GenerateMipmap(target);
            }

            let min_filter = if generate_mipmaps { gl::LINEAR_MIPMAP_LINEAR } else { gl::LINEAR };
            gl::TexParameteri(target, gl::TEXTURE_MIN_FILTER, min_filter as i32);

            gl::BindTexture(target, 0);

            Ok(Self {
                texture,
                kind
            })
        }
    }

    pub fn bind(&self, sampler_index: usize) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + sampler_index as u32);
            gl::BindTexture(self.kind.to_texture_target(), self.texture);
        }
    }

    pub fn set_max_anisotropy(&self) {
        unsafe {
            let mut aniso = 0.0;
            gl::GetFloatv(gl::MAX_TEXTURE_MAX_ANISOTROPY_EXT, &mut aniso);
            gl::TexParameterf(gl::TEXTURE_2D, gl::TEXTURE_MAX_ANISOTROPY_EXT, aniso);
        }
    }
}

impl Drop for GpuTexture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.texture);
        }
    }
}

