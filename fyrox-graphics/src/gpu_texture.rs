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
    core::{color::Color, Downcast},
    error::FrameworkError,
};
use bytemuck::Pod;

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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PixelKind {
    R32F,
    R32UI,
    R16F,
    D32F,
    D16,
    D24S8,
    RGBA8,
    SRGBA8,
    RGB8,
    SRGB8,
    BGRA8,
    BGR8,
    RG8,
    LA8,
    LA16,
    RG16,
    R8,
    L8,
    L16,
    R8UI,
    R16,
    RGB16,
    RGBA16,
    DXT1RGB,
    DXT1RGBA,
    DXT3RGBA,
    DXT5RGBA,
    RGB32F,
    RGBA32F,
    RGB16F,
    RGBA16F,
    R8RGTC,
    RG8RGTC,
    R11G11B10F,
    RGB10A2,
}

pub enum PixelElementKind {
    Float,
    NormalizedUnsignedInteger,
    Integer,
    UnsignedInteger,
}

impl PixelKind {
    pub fn unpack_alignment(self) -> Option<i32> {
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

#[derive(Default, Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Debug)]
#[repr(u32)]
pub enum MagnificationFilter {
    Nearest,
    #[default]
    Linear,
}

#[derive(Default, Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Debug)]
pub enum MinificationFilter {
    Nearest,
    NearestMipMapNearest,
    NearestMipMapLinear,
    #[default]
    Linear,
    LinearMipMapNearest,
    LinearMipMapLinear,
}

#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
pub enum WrapMode {
    #[default]
    Repeat,
    ClampToEdge,
    ClampToBorder,
    MirroredRepeat,
    MirrorClampToEdge,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Coordinate {
    S,
    T,
    R,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum CubeMapFace {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

pub struct GpuTextureDescriptor<'a> {
    pub kind: GpuTextureKind,
    pub pixel_kind: PixelKind,
    pub min_filter: MinificationFilter,
    pub mag_filter: MagnificationFilter,
    pub mip_count: usize,
    pub s_wrap_mode: WrapMode,
    pub t_wrap_mode: WrapMode,
    pub r_wrap_mode: WrapMode,
    pub anisotropy: f32,
    pub data: Option<&'a [u8]>,
    pub base_level: usize,
    pub max_level: usize,
    pub min_lod: f32,
    pub max_lod: f32,
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

pub trait GpuTexture: Downcast {
    fn set_anisotropy(&mut self, anisotropy: f32);
    fn anisotropy(&self) -> f32;
    fn set_minification_filter(&mut self, min_filter: MinificationFilter);
    fn minification_filter(&self) -> MinificationFilter;
    fn set_magnification_filter(&mut self, mag_filter: MagnificationFilter);
    fn magnification_filter(&self) -> MagnificationFilter;
    fn set_wrap(&mut self, coordinate: Coordinate, wrap: WrapMode);
    fn wrap_mode(&self, coordinate: Coordinate) -> WrapMode;
    fn set_border_color(&mut self, color: Color);
    fn set_data(
        &mut self,
        kind: GpuTextureKind,
        pixel_kind: PixelKind,
        mip_count: usize,
        data: Option<&[u8]>,
    ) -> Result<(), FrameworkError>;
    fn get_image(&self, level: usize) -> Vec<u8>;
    fn read_pixels(&self) -> Vec<u8>;
    fn kind(&self) -> GpuTextureKind;
    fn pixel_kind(&self) -> PixelKind;
    fn set_base_level(&mut self, level: usize);
    fn base_level(&self) -> usize;
    fn set_max_level(&mut self, level: usize);
    fn max_level(&self) -> usize;
    fn set_min_lod(&mut self, min_lod: f32);
    fn min_lod(&self) -> f32;
    fn set_max_lod(&mut self, max_lod: f32);
    fn max_lod(&self) -> f32;
    fn set_lod_bias(&mut self, bias: f32);
    fn lod_bias(&self) -> f32;
}

impl dyn GpuTexture {
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
