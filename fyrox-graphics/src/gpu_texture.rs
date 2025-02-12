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

use crate::{
    core::{color::Color, Downcast},
    define_shared_wrapper,
    error::FrameworkError,
};
use bytemuck::Pod;

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
    (x + 3) / 4
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

/// The texture magnification function is used when the pixel being textured maps to an area
/// less than or equal to one texture element.
#[derive(Default, Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Debug)]
#[repr(u32)]
pub enum MagnificationFilter {
    /// Returns the value of the texture element that is nearest to the center of the pixel
    /// being textured.
    Nearest,
    /// Returns the weighted average of the four texture elements that are closest to the
    /// center of the pixel being textured.
    #[default]
    Linear,
}

/// The texture minifying function is used whenever the pixel being textured maps to an area
/// greater than one texture element.
#[derive(Default, Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Debug)]
pub enum MinificationFilter {
    /// Returns the value of the texture element that is nearest to the center of the pixel
    /// being textured.
    Nearest,
    /// Chooses the mipmap that most closely matches the size of the pixel being textured and
    /// uses the Nearest criterion (the texture element nearest to the center of the pixel)
    /// to produce a texture value.
    NearestMipMapNearest,
    /// Chooses the two mipmaps that most closely match the size of the pixel being textured
    /// and uses the Nearest criterion (the texture element nearest to the center of the pixel)
    /// to produce a texture value from each mipmap. The final texture value is a weighted average
    /// of those two values.
    NearestMipMapLinear,
    /// Returns the weighted average of the four texture elements that are closest to the
    /// center of the pixel being textured.
    #[default]
    Linear,
    /// Chooses the mipmap that most closely matches the size of the pixel being textured and
    /// uses the Linear criterion (a weighted average of the four texture elements that are
    /// closest to the center of the pixel) to produce a texture value.
    LinearMipMapNearest,
    /// Chooses the two mipmaps that most closely match the size of the pixel being textured
    /// and uses the Linear criterion (a weighted average of the four texture elements that
    /// are closest to the center of the pixel) to produce a texture value from each mipmap.
    /// The final texture value is a weighted average of those two values.
    LinearMipMapLinear,
}

/// Defines a law of texture coordinate modification.
#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
pub enum WrapMode {
    /// Causes the integer part of a coordinate to be ignored; GPU uses only the fractional part,
    /// thereby creating a repeating pattern.
    #[default]
    Repeat,
    /// Causes a coordinates to be clamped to the range, where N is the size of the texture
    /// in the direction of clamping
    ClampToEdge,
    /// Evaluates a coordinates in a similar manner to ClampToEdge. However, in cases where clamping
    /// would have occurred in ClampToEdge mode, the fetched texel data is substituted with the values
    /// specified by border color.
    ClampToBorder,
    /// Causes the coordinate to be set to the fractional part of the texture coordinate if the integer
    /// part of coordinate is even; if the integer part of coordinate is odd, then the coordinate texture
    /// coordinate is set to 1-frac, where frac represents the fractional part of coordinate.
    MirroredRepeat,
    /// Causes a coordinate to be repeated as for MirroredRepeat for one repetition of the texture, at
    /// which point the coordinate to be clamped as in ClampToEdge.
    MirrorClampToEdge,
}

/// Texture coordinate.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Coordinate {
    /// S coordinate, similar to X axis.
    S,
    /// T coordinate, similar to Y axis.
    T,
    /// R coordinate, similar to Z axis.
    R,
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
    /// Minification filter of the texture. See [`MinificationFilter`] docs for more info.
    pub min_filter: MinificationFilter,
    /// Magnification filter of the texture. See [`MagnificationFilter`] docs for more info.
    pub mag_filter: MagnificationFilter,
    /// Total number of mips in the texture. Texture data must contain at least this number of
    /// mips.
    pub mip_count: usize,
    /// S coordinate wrap mode. See [`WrapMode`] docs for more info.
    pub s_wrap_mode: WrapMode,
    /// T coordinate wrap mode. See [`WrapMode`] docs for more info.
    pub t_wrap_mode: WrapMode,
    /// R coordinate wrap mode. See [`WrapMode`] docs for more info.
    pub r_wrap_mode: WrapMode,
    /// Anisotropy level of the texture. Default is 1.0. Max number is usually depends on the
    /// GPU, but the cap is 16.0 on pretty much any platform. This number should be a power of two.
    pub anisotropy: f32,
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
    /// Sets the minimum level-of-detail parameter. This floating-point value limits the selection
    /// of highest resolution mipmap (lowest mipmap level). The initial value is -1000.0.
    pub min_lod: f32,
    /// Sets the maximum level-of-detail parameter. This floating-point value limits the selection
    /// of the lowest resolution mipmap (highest mipmap level). The initial value is 1000.0.
    pub max_lod: f32,
    /// Specifies a fixed bias value that is to be added to the level-of-detail parameter for the
    /// texture before texture sampling. The specified value is added to the shader-supplied bias
    /// value (if any) and subsequently clamped into the implementation-defined range
    /// `−bias_max..bias_max`, where `bias_max` is the value that can be fetched from the current
    /// graphics server. The initial value is 0.0.
    pub lod_bias: f32,
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
            min_filter: Default::default(),
            mag_filter: Default::default(),
            mip_count: 1,
            s_wrap_mode: Default::default(),
            t_wrap_mode: Default::default(),
            r_wrap_mode: Default::default(),
            anisotropy: 1.0,
            data: None,
            base_level: 0,
            max_level: 1000,
            min_lod: -1000.0,
            max_lod: 1000.0,
            lod_bias: 0.0,
        }
    }
}

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
///         GpuTexture, GpuTextureDescriptor, GpuTextureKind, MagnificationFilter,
///         MinificationFilter, PixelKind, WrapMode,
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
///         min_filter: MinificationFilter::Nearest,
///         mag_filter: MagnificationFilter::Nearest,
///         mip_count: 1,
///         s_wrap_mode: WrapMode::Repeat,
///         t_wrap_mode: WrapMode::Repeat,
///         r_wrap_mode: WrapMode::Repeat,
///         anisotropy: 1.0,
///         // Opaque red pixel.
///         data: Some(&[255, 0, 0, 255]),
///         // Take the defaults for the rest of parameters.
///         ..Default::default()
///     })
/// }
/// ```
pub trait GpuTextureTrait: Downcast {
    /// Max samples for anisotropic filtering. Default value is 16.0 (max). However, real value passed
    /// to GPU will be clamped to maximum supported by current GPU. To disable anisotropic filtering
    /// set this to 1.0. Typical values are 2.0, 4.0, 8.0, 16.0.
    fn set_anisotropy(&self, anisotropy: f32);

    /// Returns current anisotropy level.
    fn anisotropy(&self) -> f32;

    /// Sets new minification filter. It is used when texture becomes smaller. See [`MinificationFilter`]
    /// docs for more info.
    fn set_minification_filter(&self, min_filter: MinificationFilter);

    /// Returns current minification filter.
    fn minification_filter(&self) -> MinificationFilter;

    /// Sets new magnification filter. It is used when texture is "stretching". See [`MagnificationFilter`]
    /// docs for more info.
    fn set_magnification_filter(&self, mag_filter: MagnificationFilter);

    /// Returns current magnification filter.
    fn magnification_filter(&self) -> MagnificationFilter;

    /// Sets new wrap mode for the given coordinate. See [`WrapMode`] for more info.
    fn set_wrap(&self, coordinate: Coordinate, wrap: WrapMode);

    /// Returns current wrap mode for the given coordinate.
    fn wrap_mode(&self, coordinate: Coordinate) -> WrapMode;

    /// Sets border color of the texture. Works together with [`WrapMode::ClampToBorder`] and
    /// essentially forces the GPU to use the given color when it tries to read outside the texture
    /// bounds.
    fn set_border_color(&self, color: Color);

    /// Sets the new data of the texture. This method is also able to change the kind of the texture
    /// and its pixel kind.
    fn set_data(
        &self,
        kind: GpuTextureKind,
        pixel_kind: PixelKind,
        mip_count: usize,
        data: Option<&[u8]>,
    ) -> Result<(), FrameworkError>;

    /// Reads the texture data at the given mip level. This method could block current thread until
    /// the data comes from GPU to CPU side.
    fn get_image(&self, level: usize) -> Vec<u8>;

    /// Reads texture pixels.
    fn read_pixels(&self) -> Vec<u8>;

    /// Returns kind of the texture.
    fn kind(&self) -> GpuTextureKind;

    /// Returns pixel kind of the texture.
    fn pixel_kind(&self) -> PixelKind;

    /// Specifies the index of the lowest defined mipmap level. Keep in mind, that the texture data
    /// should provide the actual mip map level defined by the provided value, otherwise the
    /// rendering will be incorrect (probably just black on majority of implementations) and glitchy.
    fn set_base_level(&self, level: usize);

    /// Returns the index of the lowest defined mipmap level.
    fn base_level(&self) -> usize;

    /// Sets the index of the highest defined mipmap level. Keep in mind, that the texture data
    /// should provide the actual mip map level defined by the provided value, otherwise the
    /// rendering will be incorrect (probably just black on majority of implementations) and glitchy.
    fn set_max_level(&self, level: usize);

    /// Returns the index of the highest defined mipmap level.
    fn max_level(&self) -> usize;

    /// Sets the minimum level-of-detail parameter. This floating-point value limits the selection
    /// of highest resolution mipmap (lowest mipmap level). The initial value is -1000.0.
    fn set_min_lod(&self, min_lod: f32);

    /// Returns the minimum level-of-detail parameter. See [`Self::set_min_lod`] for more info.
    fn min_lod(&self) -> f32;

    /// Sets the maximum level-of-detail parameter. This floating-point value limits the selection
    /// of the lowest resolution mipmap (highest mipmap level). The initial value is 1000.
    fn set_max_lod(&self, max_lod: f32);

    /// Returns the maximum level-of-detail parameter. See [`Self::set_max_lod`] for more info.
    fn max_lod(&self) -> f32;

    /// Specifies a fixed bias value that is to be added to the level-of-detail parameter for the
    /// texture before texture sampling. The specified value is added to the shader-supplied bias
    /// value (if any) and subsequently clamped into the implementation-defined range
    /// `−bias_max..bias_max`, where `bias_max` is the value that can be fetched from the current
    /// graphics server. The initial value is 0.0.
    fn set_lod_bias(&self, bias: f32);

    /// Returns a fixed bias value that is to be added to the level-of-detail parameter for the
    /// texture before texture sampling. See [`Self::set_lod_bias`] for more info.
    fn lod_bias(&self) -> f32;
}

impl dyn GpuTextureTrait {
    /// Reads the pixels at the given mip level and reinterprets them using the given type.
    pub fn get_image_of_type<T: Pod>(&self, level: usize) -> Vec<T> {
        let mut bytes = self.get_image(level);

        let typed = unsafe {
            Vec::<T>::from_raw_parts(
                bytes.as_mut_ptr() as *mut T,
                bytes.len() / size_of::<T>(),
                bytes.capacity() / size_of::<T>(),
            )
        };

        std::mem::forget(bytes);

        typed
    }

    /// Reads the pixels and reinterprets them using the given type.
    pub fn read_pixels_of_type<T>(&self) -> Vec<T>
    where
        T: Pod,
    {
        let mut bytes = self.read_pixels();
        let typed = unsafe {
            Vec::<T>::from_raw_parts(
                bytes.as_mut_ptr() as *mut T,
                bytes.len() / size_of::<T>(),
                bytes.capacity() / size_of::<T>(),
            )
        };
        std::mem::forget(bytes);
        typed
    }
}

define_shared_wrapper!(GpuTexture<dyn GpuTextureTrait>);
