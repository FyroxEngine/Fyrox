// Keep this for now, some texture kind might be used in future.
#![allow(dead_code)]

use crate::{
    core::color::Color,
    renderer::{
        error::RendererError,
        framework::{gl, gl::types::GLuint, state::State},
    },
    resource::texture::TextureKind,
    utils::log::Log,
};
use std::{ffi::c_void, marker::PhantomData};

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
            Self::Line { .. } => gl::TEXTURE_1D,
            Self::Rectangle { .. } => gl::TEXTURE_2D,
            Self::Cube { .. } => gl::TEXTURE_CUBE_MAP,
            Self::Volume { .. } => gl::TEXTURE_3D,
        }
    }
}

#[derive(Copy, Clone)]
pub enum PixelKind {
    F32,
    F16,
    D32,
    D16,
    D24S8,
    RGBA8,
    RGB8,
    RG8,
    R8,
}

impl From<TextureKind> for PixelKind {
    fn from(texture_kind: TextureKind) -> Self {
        match texture_kind {
            TextureKind::R8 => Self::R8,
            TextureKind::RGB8 => Self::RGB8,
            TextureKind::RGBA8 => Self::RGBA8,
        }
    }
}

pub struct GpuTexture {
    texture: GLuint,
    kind: GpuTextureKind,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

impl PixelKind {
    fn size_bytes(self) -> usize {
        match self {
            Self::RGBA8 | Self::D24S8 | Self::D32 | Self::F32 => 4,
            Self::RGB8 => 3,
            Self::RG8 | Self::D16 | Self::F16 => 2,
            Self::R8 => 1,
        }
    }

    fn unpack_alignment(self) -> i32 {
        match self {
            Self::RGBA8 | Self::RGB8 | Self::D24S8 | Self::D32 | Self::F32 => 4,
            Self::RG8 | Self::D16 | Self::F16 => 2,
            Self::R8 => 1,
        }
    }
}

#[derive(Copy, Clone)]
pub enum MagnificationFilter {
    Nearest,
    Linear,
}

impl MagnificationFilter {
    pub fn into_gl_value(self) -> i32 {
        (match self {
            Self::Nearest => gl::NEAREST,
            Self::Linear => gl::LINEAR,
        }) as i32
    }
}

#[derive(Copy, Clone)]
pub enum MininificationFilter {
    Nearest,
    Linear,
    LinearMip,
}

impl MininificationFilter {
    pub fn into_gl_value(self) -> i32 {
        (match self {
            Self::Nearest => gl::NEAREST,
            Self::Linear => gl::LINEAR,
            Self::LinearMip => gl::LINEAR_MIPMAP_LINEAR,
        }) as i32
    }
}

#[derive(Copy, Clone)]
pub enum WrapMode {
    Repeat,
    ClampToEdge,
    ClampToBorder,
}

impl WrapMode {
    pub fn into_gl_value(self) -> i32 {
        (match self {
            Self::Repeat => gl::REPEAT,
            Self::ClampToEdge => gl::CLAMP_TO_EDGE,
            Self::ClampToBorder => gl::CLAMP_TO_BORDER,
        }) as i32
    }
}

#[derive(Copy, Clone)]
pub enum Coordinate {
    S,
    T,
}

impl Coordinate {
    pub fn into_gl_value(self) -> u32 {
        match self {
            Self::S => gl::TEXTURE_WRAP_S,
            Self::T => gl::TEXTURE_WRAP_T,
        }
    }
}

pub struct TextureBinding<'a> {
    texture: &'a mut GpuTexture,
}

#[derive(Copy, Clone)]
pub enum CubeMapFace {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

impl CubeMapFace {
    pub fn into_gl_value(self) -> u32 {
        match self {
            Self::PositiveX => gl::TEXTURE_CUBE_MAP_POSITIVE_X,
            Self::NegativeX => gl::TEXTURE_CUBE_MAP_NEGATIVE_X,
            Self::PositiveY => gl::TEXTURE_CUBE_MAP_POSITIVE_Y,
            Self::NegativeY => gl::TEXTURE_CUBE_MAP_NEGATIVE_Y,
            Self::PositiveZ => gl::TEXTURE_CUBE_MAP_POSITIVE_Z,
            Self::NegativeZ => gl::TEXTURE_CUBE_MAP_NEGATIVE_Z,
        }
    }
}

impl<'a> TextureBinding<'a> {
    pub fn set_max_anisotropy(self) -> Self {
        unsafe {
            let mut aniso = 0.0;
            gl::GetFloatv(gl::MAX_TEXTURE_MAX_ANISOTROPY_EXT, &mut aniso);
            gl::TexParameterf(gl::TEXTURE_2D, gl::TEXTURE_MAX_ANISOTROPY_EXT, aniso);
        }
        self
    }

    pub fn set_minification_filter(self, min_filter: MininificationFilter) -> Self {
        unsafe {
            gl::TexParameteri(
                self.texture.kind.to_texture_target(),
                gl::TEXTURE_MIN_FILTER,
                min_filter.into_gl_value(),
            );
        }
        self
    }

    pub fn set_magnification_filter(self, mag_filter: MagnificationFilter) -> Self {
        unsafe {
            gl::TexParameteri(
                self.texture.kind.to_texture_target(),
                gl::TEXTURE_MAG_FILTER,
                mag_filter.into_gl_value(),
            );
        }
        self
    }

    pub fn set_wrap(self, coordinate: Coordinate, wrap: WrapMode) -> Self {
        unsafe {
            gl::TexParameteri(
                self.texture.kind.to_texture_target(),
                coordinate.into_gl_value(),
                wrap.into_gl_value(),
            );
        }
        self
    }

    pub fn set_border_color(self, color: Color) -> Self {
        unsafe {
            let color = color.as_frgba();
            let color = [color.x, color.y, color.z, color.w];
            gl::TexParameterfv(
                self.texture.kind.to_texture_target(),
                gl::TEXTURE_BORDER_COLOR,
                color.as_ptr(),
            );
        }
        self
    }

    pub fn generate_mip_maps(self) -> Self {
        unsafe {
            gl::GenerateMipmap(self.texture.kind.to_texture_target());
        }
        self
    }
}

impl GpuTexture {
    /// Creates new GPU texture of specified kind
    ///
    /// # Data layout
    ///
    /// In case of Cube texture, `bytes` should contain all 6 cube faces ordered like so,
    /// +X, -X, +Y, -Y, +Z, -Z
    ///
    /// Produced texture can be used as render target for framebuffer, in this case `data`
    /// parameter can be None.
    pub fn new(
        state: &mut State,
        kind: GpuTextureKind,
        pixel_kind: PixelKind,
        data: Option<&[u8]>,
    ) -> Result<Self, RendererError> {
        let bytes_per_pixel = pixel_kind.size_bytes();

        let desired_byte_count = match kind {
            GpuTextureKind::Line { length } => length * bytes_per_pixel,
            GpuTextureKind::Rectangle { width, height } => width * height * bytes_per_pixel,
            GpuTextureKind::Cube { width, height } => 6 * width * height * bytes_per_pixel,
            GpuTextureKind::Volume {
                width,
                height,
                depth,
            } => width * height * depth * bytes_per_pixel,
        };

        if let Some(data) = data {
            if data.len() != desired_byte_count {
                return Err(RendererError::InvalidTextureData {
                    expected_data_size: desired_byte_count,
                    actual_data_size: data.len(),
                });
            }
        }

        let target = kind.to_texture_target();

        unsafe {
            let mut texture = 0;
            gl::GenTextures(1, &mut texture);

            state.set_texture(0, target, texture);

            let (type_, format, internal_format) = match pixel_kind {
                PixelKind::F32 => (gl::FLOAT, gl::RED, gl::R32F),
                PixelKind::F16 => (gl::FLOAT, gl::RED, gl::R16F),
                PixelKind::D32 => (gl::FLOAT, gl::DEPTH_COMPONENT, gl::DEPTH_COMPONENT32),
                PixelKind::D16 => (gl::FLOAT, gl::DEPTH_COMPONENT, gl::DEPTH_COMPONENT16),
                PixelKind::D24S8 => (
                    gl::UNSIGNED_INT_24_8,
                    gl::DEPTH_STENCIL,
                    gl::DEPTH24_STENCIL8,
                ),
                PixelKind::RGBA8 => (gl::UNSIGNED_BYTE, gl::RGBA, gl::RGBA8),
                PixelKind::RGB8 => (gl::UNSIGNED_BYTE, gl::RGB, gl::RGB8),
                PixelKind::RG8 => (gl::UNSIGNED_BYTE, gl::RG, gl::RG8),
                PixelKind::R8 => (gl::UNSIGNED_BYTE, gl::RED, gl::R8),
            };

            gl::PixelStorei(gl::UNPACK_ALIGNMENT, pixel_kind.unpack_alignment());

            let pixels = match data {
                None => std::ptr::null(),
                Some(data) => data.as_ptr() as *const c_void,
            };

            match kind {
                GpuTextureKind::Line { length } => {
                    gl::TexImage1D(
                        gl::TEXTURE_1D,
                        0,
                        internal_format as i32,
                        length as i32,
                        0,
                        format,
                        type_,
                        pixels,
                    );
                }
                GpuTextureKind::Rectangle { width, height } => {
                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        internal_format as i32,
                        width as i32,
                        height as i32,
                        0,
                        format,
                        type_,
                        pixels,
                    );
                }
                GpuTextureKind::Cube { width, height } => {
                    for face in 0..6 {
                        let bytes_per_face = width * height * bytes_per_pixel;

                        let begin = face * bytes_per_face;
                        let end = (face + 1) * bytes_per_face;

                        let face_pixels = match data {
                            None => std::ptr::null(),
                            Some(data) => data[begin..end].as_ptr() as *const c_void,
                        };

                        gl::TexImage2D(
                            gl::TEXTURE_CUBE_MAP_POSITIVE_X + face as u32,
                            0,
                            internal_format as i32,
                            width as i32,
                            height as i32,
                            0,
                            format,
                            type_,
                            face_pixels,
                        );
                    }
                }
                GpuTextureKind::Volume {
                    width,
                    height,
                    depth,
                } => {
                    gl::TexImage3D(
                        gl::TEXTURE_3D,
                        0,
                        internal_format as i32,
                        width as i32,
                        height as i32,
                        depth as i32,
                        0,
                        format,
                        type_,
                        pixels,
                    );
                }
            }

            gl::TexParameteri(target, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(target, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);

            state.set_texture(0, target, 0);

            Log::writeln(format!("GL texture {} was created!", texture));

            Ok(Self {
                texture,
                kind,
                thread_mark: PhantomData,
            })
        }
    }

    pub fn bind_mut(&mut self, state: &mut State, sampler_index: usize) -> TextureBinding<'_> {
        state.set_texture(sampler_index, self.kind.to_texture_target(), self.texture);
        TextureBinding { texture: self }
    }

    pub fn bind(&self, state: &mut State, sampler_index: usize) {
        state.set_texture(sampler_index, self.kind.to_texture_target(), self.texture);
    }

    pub fn kind(&self) -> GpuTextureKind {
        self.kind
    }

    pub fn id(&self) -> u32 {
        self.texture
    }
}

impl Drop for GpuTexture {
    fn drop(&mut self) {
        unsafe {
            Log::writeln(format!("GL texture {} was destroyed!", self.texture));

            gl::DeleteTextures(1, &self.texture);
        }
    }
}
