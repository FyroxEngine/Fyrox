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

//! Texture is an image that used to fill faces to add details to them. It could also be used as a
//! generic and mostly unlimited capacity storage for arbitrary data.

#![warn(missing_docs)]

use crate::{define_shared_wrapper, error::FrameworkError};
use fyrox_core::define_as_any_trait;

/// A kind of GPU texture.
#[derive(Copy, Clone)]
pub enum GpuTextureKind {
    /// 1D texture.
    Line {
        /// Length of the texture.
        length: usize,
    },
    /// 2D texture.
    Rectangle {
        /// Width of the texture.
        width: usize,
        /// Height of the texture.
        height: usize,
    },
    /// Six 2D textures forming a cube.
    Cube {
        /// Width of the texture.
        width: usize,
        /// Height of the texture.
        height: usize,
    },
    /// Volumetric texture that consists of `depth` textures with `width x height` size.
    Volume {
        /// Width of the texture.
        width: usize,
        /// Height of the texture.
        height: usize,
        /// Depth of the texture.
        depth: usize,
    },
}

/// Pixel kind of GPU texture.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PixelKind {
    /// Floating point 32-bit pixel.
    R32F,
    /// Unsigned integer 32-bit pixel.
    R32UI,
    /// Floating point 16-bit pixel.
    R16F,
    /// Floating point 32-bit depth pixel.
    D32F,
    /// Integer 16-bit depth pixel.
    D16,
    /// Integer 24-bit depth pixel + 8-bit stencil.
    D24S8,
    /// Red, Green, Blue, Alpha; all by 8-bit.
    RGBA8,
    /// Red, Green, Blue, Alpha in sRGB color space; all by 8-bit.
    SRGBA8,
    /// Red, Green, Blue; all by 8-bit.
    RGB8,
    /// Red, Green, Blue in sRGB color space; all by 8-bit.
    SRGB8,
    /// Blue, Green, Red, Alpha; all by 8-bit.
    BGRA8,
    /// Blue, Green, Red; all by 8-bit.
    BGR8,
    /// Red, Green; all by 8-bit.
    RG8,
    /// Luminance, Alpha; all by 8-bit.
    LA8,
    /// Luminance, Alpha; all by 16-bit.
    LA16,
    /// Red, Green; all by 16-bit.
    RG16,
    /// Red, Green; 16-bit.
    R8,
    /// Luminance; 8-bit.
    L8,
    /// Luminance; 16-bit.
    L16,
    /// Red, unsigned integer; 8-bit.
    R8UI,
    /// Red, signed integer; 16-bit.
    R16,
    /// Red, Green, Blue; all by 16-bit.
    RGB16,
    /// Red, Green, Blue, Alpha; all by 8-bit.
    RGBA16,
    /// Compressed S3TC DXT1 RGB.
    DXT1RGB,
    /// Compressed S3TC DXT1 RGBA.
    DXT1RGBA,
    /// Compressed S3TC DXT3 RGBA.
    DXT3RGBA,
    /// Compressed S3TC DXT5 RGBA.
    DXT5RGBA,
    /// Floating-point RGB texture with 32-bit depth.
    RGB32F,
    /// Floating-point RGBA texture with 32-bit depth.
    RGBA32F,
    /// Floating-point RGB texture with 16-bit depth.
    RGB16F,
    /// Floating-point RGBA texture with 16-bit depth.
    RGBA16F,
    /// Compressed R8 texture (RGTC).
    R8RGTC,
    /// Compressed RG8 texture (RGTC).
    RG8RGTC,
    /// Floating-point RGB texture with 11-bit for Red and Green channels, 10-bit for Blue channel.
    R11G11B10F,
    /// Red, Green, Blue (8-bit) + Alpha (2-bit).
    RGB10A2,
}

/// Element kind of pixel.
pub enum PixelElementKind {
    /// Floating-point pixel.
    Float,
    /// Normalized unsigned integer.
    NormalizedUnsignedInteger,
    /// Integer.
    Integer,
    /// Unsigned integer.
    UnsignedInteger,
}

impl PixelKind {
    pub(crate) fn unpack_alignment(self) -> Option<i32> {
        match self {
            Self::RGBA16
            | Self::RGBA16F
            | Self::RGB16
            | Self::RGB16F
            | Self::RGBA32F
            | Self::RGB32F
            | Self::RGBA8
            | Self::SRGBA8
            | Self::BGRA8
            | Self::RG16
            | Self::LA16
            | Self::D24S8
            | Self::D32F
            | Self::R32F
            | Self::R32UI
            | Self::RGB10A2 => Some(4),
            Self::RG8 | Self::LA8 | Self::D16 | Self::R16F | Self::L16 | Self::R16 => Some(2),
            Self::R8
            | Self::L8
            | Self::R8UI
            | Self::SRGB8
            | Self::RGB8
            | Self::BGR8
            | Self::R11G11B10F => Some(1),
            Self::DXT1RGB
            | Self::DXT1RGBA
            | Self::DXT3RGBA
            | Self::DXT5RGBA
            | Self::R8RGTC
            | Self::RG8RGTC => None,
        }
    }

    /// Returns `true` if the pixel kind is compressed, `false` - otherwise.
    pub fn is_compressed(self) -> bool {
        match self {
            Self::DXT1RGB
            | Self::DXT1RGBA
            | Self::DXT3RGBA
            | Self::DXT5RGBA
            | Self::R8RGTC
            | Self::RG8RGTC => true,
            // Explicit match for rest of formats instead of _ will help to not forget
            // to add new entry here.
            Self::RGBA16
            | Self::RGBA16F
            | Self::RGB16
            | Self::RGB16F
            | Self::RGBA8
            | Self::SRGBA8
            | Self::RGB8
            | Self::SRGB8
            | Self::BGRA8
            | Self::BGR8
            | Self::RG16
            | Self::R16
            | Self::D24S8
            | Self::D32F
            | Self::R32F
            | Self::R32UI
            | Self::RG8
            | Self::D16
            | Self::R16F
            | Self::R8
            | Self::R8UI
            | Self::RGB32F
            | Self::RGBA32F
            | Self::R11G11B10F
            | Self::RGB10A2
            | Self::L8
            | Self::LA8
            | Self::L16
            | Self::LA16 => false,
        }
    }

    /// Returns element kind of the pixel.
    pub fn element_kind(self) -> PixelElementKind {
        match self {
            Self::R32F
            | Self::R16F
            | Self::RGB32F
            | Self::RGBA32F
            | Self::RGBA16F
            | Self::RGB16F
            | Self::D32F
            | Self::R11G11B10F => PixelElementKind::Float,
            Self::D16
            | Self::D24S8
            | Self::RGBA8
            | Self::SRGBA8
            | Self::RGB8
            | Self::SRGB8
            | Self::BGRA8
            | Self::BGR8
            | Self::RG8
            | Self::RG16
            | Self::R8
            | Self::R16
            | Self::RGB16
            | Self::RGBA16
            | Self::DXT1RGB
            | Self::DXT1RGBA
            | Self::DXT3RGBA
            | Self::DXT5RGBA
            | Self::R8RGTC
            | Self::RG8RGTC
            | Self::RGB10A2
            | Self::LA8
            | Self::L8
            | Self::LA16
            | Self::L16 => PixelElementKind::NormalizedUnsignedInteger,
            Self::R8UI | Self::R32UI => PixelElementKind::UnsignedInteger,
        }
    }
}

fn ceil_div_4(x: usize) -> usize {
    x.div_ceil(4)
}

/// Calculates size in bytes of a volume texture using the given size of the texture and its pixel
/// kind.
pub fn image_3d_size_bytes(
    pixel_kind: PixelKind,
    width: usize,
    height: usize,
    depth: usize,
) -> usize {
    let pixel_count = width * height * depth;
    match pixel_kind {
        PixelKind::RGBA32F => 16 * pixel_count,
        PixelKind::RGB32F => 12 * pixel_count,
        PixelKind::RGBA16 | PixelKind::RGBA16F => 8 * pixel_count,
        PixelKind::RGB16 | PixelKind::RGB16F => 6 * pixel_count,
        PixelKind::RGBA8
        | PixelKind::SRGBA8
        | PixelKind::BGRA8
        | PixelKind::RG16
        | PixelKind::LA16
        | PixelKind::D24S8
        | PixelKind::D32F
        | PixelKind::R32F
        | PixelKind::R32UI
        | PixelKind::R11G11B10F
        | PixelKind::RGB10A2 => 4 * pixel_count,
        PixelKind::RGB8 | PixelKind::SRGB8 | PixelKind::BGR8 => 3 * pixel_count,
        PixelKind::RG8
        | PixelKind::LA8
        | PixelKind::R16
        | PixelKind::L16
        | PixelKind::D16
        | PixelKind::R16F => 2 * pixel_count,
        PixelKind::R8 | PixelKind::L8 | PixelKind::R8UI => pixel_count,
        PixelKind::DXT1RGB | PixelKind::DXT1RGBA | PixelKind::R8RGTC => {
            let block_size = 8;
            ceil_div_4(width) * ceil_div_4(height) * ceil_div_4(depth) * block_size
        }
        PixelKind::DXT3RGBA | PixelKind::DXT5RGBA | PixelKind::RG8RGTC => {
            let block_size = 16;
            ceil_div_4(width) * ceil_div_4(height) * ceil_div_4(depth) * block_size
        }
    }
}

/// Calculates size in bytes of a rectangular texture using the given size of the texture and its pixel
/// kind.
pub fn image_2d_size_bytes(pixel_kind: PixelKind, width: usize, height: usize) -> usize {
    let pixel_count = width * height;
    match pixel_kind {
        PixelKind::RGBA32F => 16 * pixel_count,
        PixelKind::RGB32F => 12 * pixel_count,
        PixelKind::RGBA16 | PixelKind::RGBA16F => 8 * pixel_count,
        PixelKind::RGB16 | PixelKind::RGB16F => 6 * pixel_count,
        PixelKind::RGBA8
        | PixelKind::SRGBA8
        | PixelKind::BGRA8
        | PixelKind::RG16
        | PixelKind::LA16
        | PixelKind::D24S8
        | PixelKind::D32F
        | PixelKind::R32F
        | PixelKind::R32UI
        | PixelKind::R11G11B10F
        | PixelKind::RGB10A2 => 4 * pixel_count,
        PixelKind::RGB8 | PixelKind::SRGB8 | PixelKind::BGR8 => 3 * pixel_count,
        PixelKind::RG8
        | PixelKind::LA8
        | PixelKind::R16
        | PixelKind::L16
        | PixelKind::D16
        | PixelKind::R16F => 2 * pixel_count,
        PixelKind::R8 | PixelKind::L8 | PixelKind::R8UI => pixel_count,
        PixelKind::DXT1RGB | PixelKind::DXT1RGBA | PixelKind::R8RGTC => {
            let block_size = 8;
            ceil_div_4(width) * ceil_div_4(height) * block_size
        }
        PixelKind::DXT3RGBA | PixelKind::DXT5RGBA | PixelKind::RG8RGTC => {
            let block_size = 16;
            ceil_div_4(width) * ceil_div_4(height) * block_size
        }
    }
}

/// Calculates size in bytes of a linear texture using the given size of the texture and its pixel
/// kind.
pub fn image_1d_size_bytes(pixel_kind: PixelKind, length: usize) -> usize {
    match pixel_kind {
        PixelKind::RGBA32F => 16 * length,
        PixelKind::RGB32F => 12 * length,
        PixelKind::RGBA16 | PixelKind::RGBA16F => 8 * length,
        PixelKind::RGB16 | PixelKind::RGB16F => 6 * length,
        PixelKind::RGBA8
        | PixelKind::SRGBA8
        | PixelKind::BGRA8
        | PixelKind::RG16
        | PixelKind::LA16
        | PixelKind::D24S8
        | PixelKind::D32F
        | PixelKind::R32F
        | PixelKind::R32UI
        | PixelKind::R11G11B10F
        | PixelKind::RGB10A2 => 4 * length,
        PixelKind::RGB8 | PixelKind::SRGB8 | PixelKind::BGR8 => 3 * length,
        PixelKind::RG8
        | PixelKind::LA8
        | PixelKind::L16
        | PixelKind::R16
        | PixelKind::D16
        | PixelKind::R16F => 2 * length,
        PixelKind::R8 | PixelKind::L8 | PixelKind::R8UI => length,
        PixelKind::DXT1RGB | PixelKind::DXT1RGBA | PixelKind::R8RGTC => {
            let block_size = 8;
            ceil_div_4(length) * block_size
        }
        PixelKind::DXT3RGBA | PixelKind::DXT5RGBA | PixelKind::RG8RGTC => {
            let block_size = 16;
            ceil_div_4(length) * block_size
        }
    }
}

/// Face of a cube map.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum CubeMapFace {
    /// +X face.
    PositiveX,
    /// -X face.
    NegativeX,
    /// +Y face.
    PositiveY,
    /// -Y face.
    NegativeY,
    /// +Z face.
    PositiveZ,
    /// -Z face.
    NegativeZ,
}

/// Descriptor of a texture that is used to request textures from a graphics server.
pub struct GpuTextureDescriptor<'a> {
    /// Kind of the texture. See [`GpuTextureKind`] docs for more info.
    pub kind: GpuTextureKind,
    /// Pixel kind of the texture. See [`PixelKind`] docs for more info.
    pub pixel_kind: PixelKind,
    /// Total number of mips in the texture. Texture data must contain at least this number of
    /// mips.
    pub mip_count: usize,
    /// Optional data of the texture. If present, then the total number of bytes must match the
    /// required number of bytes defined by the texture kind, pixel kind, mip count.
    pub data: Option<&'a [u8]>,
    /// Specifies the index of the lowest defined mipmap level. Keep in mind, that the texture data
    /// should provide the actual mip map level defined by the provided value, otherwise the
    /// rendering will be incorrect (probably just black on majority of implementations) and glitchy.
    pub base_level: usize,
    /// Sets the index of the highest defined mipmap level. Keep in mind, that the texture data
    /// should provide the actual mip map level defined by the provided value, otherwise the
    /// rendering will be incorrect (probably just black on majority of implementations) and glitchy.
    pub max_level: usize,
}

impl Default for GpuTextureDescriptor<'_> {
    // WARNING: Do NOT change these default values. This will affect a lot of places in the engine
    // and may potentially lead to weird behavior!
    fn default() -> Self {
        Self {
            kind: GpuTextureKind::Rectangle {
                width: 1,
                height: 1,
            },
            pixel_kind: PixelKind::RGBA8,
            mip_count: 1,
            data: None,
            base_level: 0,
            max_level: 1000,
        }
    }
}

define_as_any_trait!(GpuTextureAsAny => GpuTextureTrait);

/// Texture is an image that used to fill faces to add details to them. It could also be used as a
/// generic and mostly unlimited capacity storage for arbitrary data.
///
/// In most cases textures are just 2D images, however there are some exclusions to that - for example
/// cube maps, that may be used for environment mapping. Fyrox supports 1D, 2D, 3D and Cube textures.
///
/// ## Example
///
/// ```rust
/// use fyrox_graphics::{
///     error::FrameworkError,
///     gpu_texture::{
///         GpuTexture, GpuTextureDescriptor, GpuTextureKind, PixelKind,
///     },
///     server::GraphicsServer,
/// };
/// use std::{cell::RefCell, rc::Rc};
///
/// fn create_texture(
///     server: &dyn GraphicsServer,
/// ) -> Result<GpuTexture, FrameworkError> {
///     server.create_texture(GpuTextureDescriptor {
///         kind: GpuTextureKind::Rectangle {
///             width: 1,
///             height: 1,
///         },
///         pixel_kind: PixelKind::RGBA8,
///         mip_count: 1,
///         // Opaque red pixel.
///         data: Some(&[255, 0, 0, 255]),
///         // Take the defaults for the rest of parameters.
///         ..Default::default()
///     })
/// }
/// ```
pub trait GpuTextureTrait: GpuTextureAsAny {
    /// Sets the new data of the texture. This method is also able to change the kind of the texture
    /// and its pixel kind.
    fn set_data(
        &self,
        kind: GpuTextureKind,
        pixel_kind: PixelKind,
        mip_count: usize,
        data: Option<&[u8]>,
    ) -> Result<(), FrameworkError>;

    /// Returns kind of the texture.
    fn kind(&self) -> GpuTextureKind;

    /// Returns pixel kind of the texture.
    fn pixel_kind(&self) -> PixelKind;
}

define_shared_wrapper!(GpuTexture<dyn GpuTextureTrait>);
