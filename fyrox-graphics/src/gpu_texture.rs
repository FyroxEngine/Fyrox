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

use crate::{core::color::Color, error::FrameworkError, state::GlGraphicsServer};
use bytemuck::Pod;
use glow::{HasContext, PixelPackData, COMPRESSED_RED_RGTC1, COMPRESSED_RG_RGTC2};
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

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
    fn gl_texture_target(&self) -> u32 {
        match self {
            Self::Line { .. } => glow::TEXTURE_1D,
            Self::Rectangle { .. } => glow::TEXTURE_2D,
            Self::Cube { .. } => glow::TEXTURE_CUBE_MAP,
            Self::Volume { .. } => glow::TEXTURE_3D,
        }
    }
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

pub struct PixelDescriptor {
    pub data_type: u32,
    pub format: u32,
    pub internal_format: u32,
    pub swizzle_mask: Option<[i32; 4]>,
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

    pub fn pixel_descriptor(self) -> PixelDescriptor {
        let (data_type, format, internal_format, swizzle_mask) = match self {
            PixelKind::R32F => (glow::FLOAT, glow::RED, glow::R32F, None),
            PixelKind::R32UI => (glow::UNSIGNED_INT, glow::RED_INTEGER, glow::R32UI, None),
            PixelKind::R16F => (glow::FLOAT, glow::RED, glow::R16F, None),
            PixelKind::D32F => (
                glow::FLOAT,
                glow::DEPTH_COMPONENT,
                glow::DEPTH_COMPONENT32F,
                None,
            ),
            PixelKind::D16 => (
                glow::UNSIGNED_SHORT,
                glow::DEPTH_COMPONENT,
                glow::DEPTH_COMPONENT16,
                None,
            ),
            PixelKind::D24S8 => (
                glow::UNSIGNED_INT_24_8,
                glow::DEPTH_STENCIL,
                glow::DEPTH24_STENCIL8,
                None,
            ),
            PixelKind::RGBA8 => (glow::UNSIGNED_BYTE, glow::RGBA, glow::RGBA8, None),
            PixelKind::SRGBA8 => (glow::UNSIGNED_BYTE, glow::RGBA, glow::SRGB8_ALPHA8, None),
            PixelKind::RGB8 => (glow::UNSIGNED_BYTE, glow::RGB, glow::RGB8, None),
            PixelKind::SRGB8 => (glow::UNSIGNED_BYTE, glow::RGB, glow::SRGB8, None),
            PixelKind::RG8 => (glow::UNSIGNED_BYTE, glow::RG, glow::RG8, None),
            PixelKind::R8 => (glow::UNSIGNED_BYTE, glow::RED, glow::R8, None),
            PixelKind::R8UI => (glow::UNSIGNED_BYTE, glow::RED_INTEGER, glow::R8UI, None),
            PixelKind::BGRA8 => (glow::UNSIGNED_BYTE, glow::BGRA, glow::RGBA8, None),
            PixelKind::BGR8 => (glow::UNSIGNED_BYTE, glow::BGR, glow::RGB8, None),
            PixelKind::RG16 => (glow::UNSIGNED_SHORT, glow::RG, glow::RG16, None),
            PixelKind::R16 => (glow::UNSIGNED_SHORT, glow::RED, glow::R16, None),
            PixelKind::RGB16 => (glow::UNSIGNED_SHORT, glow::RGB, glow::RGB16, None),
            PixelKind::RGBA16 => (glow::UNSIGNED_SHORT, glow::RGBA, glow::RGBA16, None),
            PixelKind::RGB10A2 => (
                glow::UNSIGNED_INT_2_10_10_10_REV,
                glow::RGBA,
                glow::RGB10_A2,
                None,
            ),
            PixelKind::DXT1RGB => (0, 0, GL_COMPRESSED_RGB_S3TC_DXT1_EXT, None),
            PixelKind::DXT1RGBA => (0, 0, GL_COMPRESSED_RGBA_S3TC_DXT1_EXT, None),
            PixelKind::DXT3RGBA => (0, 0, GL_COMPRESSED_RGBA_S3TC_DXT3_EXT, None),
            PixelKind::DXT5RGBA => (0, 0, GL_COMPRESSED_RGBA_S3TC_DXT5_EXT, None),
            PixelKind::R8RGTC => (0, 0, COMPRESSED_RED_RGTC1, None),
            PixelKind::RG8RGTC => (0, 0, COMPRESSED_RG_RGTC2, None),
            PixelKind::RGB32F => (glow::FLOAT, glow::RGB, glow::RGB32F, None),
            PixelKind::RGBA32F => (glow::FLOAT, glow::RGBA, glow::RGBA32F, None),
            PixelKind::RGBA16F => (glow::HALF_FLOAT, glow::RGBA, glow::RGBA16F, None),
            PixelKind::RGB16F => (glow::HALF_FLOAT, glow::RGB, glow::RGB16F, None),
            PixelKind::R11G11B10F => (glow::FLOAT, glow::RGB, glow::R11F_G11F_B10F, None),
            PixelKind::L8 => (
                glow::UNSIGNED_BYTE,
                glow::RED,
                glow::R8,
                Some([
                    glow::RED as i32,
                    glow::RED as i32,
                    glow::RED as i32,
                    glow::ONE as i32,
                ]),
            ),
            PixelKind::LA8 => (
                glow::UNSIGNED_BYTE,
                glow::RG,
                glow::RG8,
                Some([
                    glow::RED as i32,
                    glow::RED as i32,
                    glow::RED as i32,
                    glow::GREEN as i32,
                ]),
            ),
            PixelKind::LA16 => (
                glow::UNSIGNED_SHORT,
                glow::RG,
                glow::RG16,
                Some([
                    glow::RED as i32,
                    glow::RED as i32,
                    glow::RED as i32,
                    glow::GREEN as i32,
                ]),
            ),
            PixelKind::L16 => (
                glow::UNSIGNED_SHORT,
                glow::RED,
                glow::R16,
                Some([
                    glow::RED as i32,
                    glow::RED as i32,
                    glow::RED as i32,
                    glow::ONE as i32,
                ]),
            ),
        };

        PixelDescriptor {
            data_type,
            format,
            internal_format,
            swizzle_mask,
        }
    }
}

pub struct GpuTexture {
    state: Weak<GlGraphicsServer>,
    texture: glow::Texture,
    kind: GpuTextureKind,
    min_filter: MinificationFilter,
    mag_filter: MagnificationFilter,
    s_wrap_mode: WrapMode,
    t_wrap_mode: WrapMode,
    r_wrap_mode: WrapMode,
    anisotropy: f32,
    pixel_kind: PixelKind,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
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

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Debug)]
#[repr(u32)]
pub enum MagnificationFilter {
    Nearest,
    Linear,
}

impl MagnificationFilter {
    pub fn into_gl_value(self) -> i32 {
        (match self {
            Self::Nearest => glow::NEAREST,
            Self::Linear => glow::LINEAR,
        }) as i32
    }
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Hash, Debug)]
#[repr(u32)]
pub enum MinificationFilter {
    Nearest = glow::NEAREST,
    NearestMipMapNearest = glow::NEAREST_MIPMAP_NEAREST,
    NearestMipMapLinear = glow::NEAREST_MIPMAP_LINEAR,
    Linear = glow::LINEAR,
    LinearMipMapNearest = glow::LINEAR_MIPMAP_NEAREST,
    LinearMipMapLinear = glow::LINEAR_MIPMAP_LINEAR,
}

impl MinificationFilter {
    pub fn into_gl_value(self) -> i32 {
        self as i32
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u32)]
pub enum WrapMode {
    Repeat = glow::REPEAT,
    ClampToEdge = glow::CLAMP_TO_EDGE,
    ClampToBorder = glow::CLAMP_TO_BORDER,
    MirroredRepeat = glow::MIRRORED_REPEAT,
    MirrorClampToEdge = glow::MIRROR_CLAMP_TO_EDGE,
}

impl WrapMode {
    pub fn into_gl_value(self) -> i32 {
        self as i32
    }
}

#[derive(Copy, Clone)]
#[repr(u32)]
pub enum Coordinate {
    S = glow::TEXTURE_WRAP_S,
    T = glow::TEXTURE_WRAP_T,
    R = glow::TEXTURE_WRAP_R,
}

impl Coordinate {
    pub fn into_gl_value(self) -> u32 {
        self as u32
    }
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
            Self::PositiveX => glow::TEXTURE_CUBE_MAP_POSITIVE_X,
            Self::NegativeX => glow::TEXTURE_CUBE_MAP_NEGATIVE_X,
            Self::PositiveY => glow::TEXTURE_CUBE_MAP_POSITIVE_Y,
            Self::NegativeY => glow::TEXTURE_CUBE_MAP_NEGATIVE_Y,
            Self::PositiveZ => glow::TEXTURE_CUBE_MAP_POSITIVE_Z,
            Self::NegativeZ => glow::TEXTURE_CUBE_MAP_NEGATIVE_Z,
        }
    }
}

const GL_COMPRESSED_RGB_S3TC_DXT1_EXT: u32 = 0x83F0;
const GL_COMPRESSED_RGBA_S3TC_DXT1_EXT: u32 = 0x83F1;
const GL_COMPRESSED_RGBA_S3TC_DXT3_EXT: u32 = 0x83F2;
const GL_COMPRESSED_RGBA_S3TC_DXT5_EXT: u32 = 0x83F3;

struct TempBinding {
    server: Rc<GlGraphicsServer>,
    unit: u32,
    target: u32,
}

impl TempBinding {
    fn new(server: Rc<GlGraphicsServer>, texture: &GpuTexture) -> Self {
        let unit = server
            .free_texture_unit()
            .expect("Texture units limit exceeded!");
        let target = texture.kind.gl_texture_target();
        server.set_texture(unit, target, Some(texture.texture));
        Self {
            server,
            unit,
            target,
        }
    }
}

impl Drop for TempBinding {
    fn drop(&mut self) {
        self.server.set_texture(self.unit, self.target, None);
    }
}

impl GpuTexture {
    /// Creates new GPU texture of specified kind. Mip count must be at least 1, it means
    /// that there is only main level of detail.
    ///
    /// # Data layout
    ///
    /// In case of Cube texture, `bytes` should contain all 6 cube faces ordered like so,
    /// +X, -X, +Y, -Y, +Z, -Z. Cube mips must follow one after another.
    ///
    /// Produced texture can be used as render target for framebuffer, in this case `data`
    /// parameter can be None.
    ///
    /// # Compressed textures
    ///
    /// For compressed textures data must contain all mips, where each mip must be 2 times
    /// smaller than previous.
    pub fn new(
        server: &GlGraphicsServer,
        kind: GpuTextureKind,
        pixel_kind: PixelKind,
        min_filter: MinificationFilter,
        mag_filter: MagnificationFilter,
        mip_count: usize,
        data: Option<&[u8]>,
    ) -> Result<Self, FrameworkError> {
        let mip_count = mip_count.max(1);

        let target = kind.gl_texture_target();

        unsafe {
            let texture = server.gl.create_texture()?;

            let mut result = Self {
                state: server.weak(),
                texture,
                kind,
                min_filter,
                mag_filter,
                s_wrap_mode: WrapMode::Repeat,
                t_wrap_mode: WrapMode::Repeat,
                r_wrap_mode: WrapMode::Repeat,
                anisotropy: 1.0,
                pixel_kind,
                thread_mark: PhantomData,
            };

            result.set_data(kind, pixel_kind, mip_count, data)?;

            server.gl.tex_parameter_i32(
                target,
                glow::TEXTURE_MAG_FILTER,
                mag_filter.into_gl_value(),
            );
            server.gl.tex_parameter_i32(
                target,
                glow::TEXTURE_MIN_FILTER,
                min_filter.into_gl_value(),
            );

            server
                .gl
                .tex_parameter_i32(target, glow::TEXTURE_MAX_LEVEL, mip_count as i32 - 1);

            server.set_texture(0, target, Default::default());

            Ok(result)
        }
    }

    pub fn bind(&self, server: &GlGraphicsServer, sampler_index: u32) {
        server.set_texture(
            sampler_index,
            self.kind.gl_texture_target(),
            Some(self.texture),
        );
    }

    fn make_temp_binding(&self) -> TempBinding {
        let server = self.state.upgrade().unwrap();
        TempBinding::new(server, self)
    }

    pub fn set_anisotropy(&mut self, anisotropy: f32) {
        let temp_binding = self.make_temp_binding();

        unsafe {
            let max = temp_binding
                .server
                .gl
                .get_parameter_f32(glow::MAX_TEXTURE_MAX_ANISOTROPY_EXT);
            temp_binding.server.gl.tex_parameter_f32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAX_ANISOTROPY_EXT,
                anisotropy.clamp(0.0, max),
            );

            // Set it to requested value, instead of hardware-limited. This will allow
            // us to check if anisotropy needs to be changed.
            self.anisotropy = anisotropy;
        }
    }

    pub fn set_minification_filter(&mut self, min_filter: MinificationFilter) {
        let temp_binding = self.make_temp_binding();

        unsafe {
            let target = self.kind.gl_texture_target();

            temp_binding.server.gl.tex_parameter_i32(
                target,
                glow::TEXTURE_MIN_FILTER,
                min_filter.into_gl_value(),
            );

            self.min_filter = min_filter;
        }
    }

    pub fn set_magnification_filter(&mut self, mag_filter: MagnificationFilter) {
        let temp_binding = self.make_temp_binding();

        unsafe {
            temp_binding.server.gl.tex_parameter_i32(
                self.kind.gl_texture_target(),
                glow::TEXTURE_MAG_FILTER,
                mag_filter.into_gl_value(),
            );

            self.mag_filter = mag_filter;
        }
    }

    pub fn set_wrap(&mut self, coordinate: Coordinate, wrap: WrapMode) {
        let temp_binding = self.make_temp_binding();

        unsafe {
            temp_binding.server.gl.tex_parameter_i32(
                self.kind.gl_texture_target(),
                coordinate.into_gl_value(),
                wrap.into_gl_value(),
            );

            match coordinate {
                Coordinate::S => self.s_wrap_mode = wrap,
                Coordinate::T => self.t_wrap_mode = wrap,
                Coordinate::R => self.r_wrap_mode = wrap,
            }
        }
    }

    pub fn set_border_color(&mut self, #[allow(unused_variables)] color: Color) {
        #[cfg(not(target_arch = "wasm32"))]
        unsafe {
            let temp_binding = self.make_temp_binding();
            let color = color.as_frgba();
            let color = [color.x, color.y, color.z, color.w];

            temp_binding.server.gl.tex_parameter_f32_slice(
                self.kind.gl_texture_target(),
                glow::TEXTURE_BORDER_COLOR,
                &color,
            );
        }
    }

    pub fn set_data(
        &mut self,
        kind: GpuTextureKind,
        pixel_kind: PixelKind,
        mip_count: usize,
        data: Option<&[u8]>,
    ) -> Result<(), FrameworkError> {
        let mip_count = mip_count.max(1);

        let mut desired_byte_count = 0;

        'mip_loop: for mip in 0..mip_count {
            match kind {
                GpuTextureKind::Line { length } => {
                    if let Some(length) = length.checked_shr(mip as u32) {
                        desired_byte_count += image_1d_size_bytes(pixel_kind, length);
                    } else {
                        break 'mip_loop;
                    }
                }
                GpuTextureKind::Rectangle { width, height } => {
                    if let (Some(width), Some(height)) = (
                        width.checked_shr(mip as u32),
                        height.checked_shr(mip as u32),
                    ) {
                        desired_byte_count += image_2d_size_bytes(pixel_kind, width, height);
                    } else {
                        break 'mip_loop;
                    }
                }
                GpuTextureKind::Cube { width, height } => {
                    if let (Some(width), Some(height)) = (
                        width.checked_shr(mip as u32),
                        height.checked_shr(mip as u32),
                    ) {
                        desired_byte_count += 6 * image_2d_size_bytes(pixel_kind, width, height);
                    } else {
                        break 'mip_loop;
                    }
                }
                GpuTextureKind::Volume {
                    width,
                    height,
                    depth,
                } => {
                    if let (Some(width), Some(height), Some(depth)) = (
                        width.checked_shr(mip as u32),
                        height.checked_shr(mip as u32),
                        depth.checked_shr(mip as u32),
                    ) {
                        desired_byte_count += image_3d_size_bytes(pixel_kind, width, height, depth);
                    } else {
                        break 'mip_loop;
                    }
                }
            };
        }

        if let Some(data) = data {
            let actual_data_size = data.len();
            if actual_data_size != desired_byte_count {
                return Err(FrameworkError::InvalidTextureData {
                    expected_data_size: desired_byte_count,
                    actual_data_size,
                });
            }
        }

        self.kind = kind;
        self.pixel_kind = pixel_kind;

        let temp_binding = self.make_temp_binding();
        let target = kind.gl_texture_target();

        unsafe {
            temp_binding.server.gl.tex_parameter_i32(
                target,
                glow::TEXTURE_MAX_LEVEL,
                mip_count as i32 - 1,
            );

            let PixelDescriptor {
                data_type,
                format,
                internal_format,
                swizzle_mask,
            } = pixel_kind.pixel_descriptor();

            let is_compressed = pixel_kind.is_compressed();

            if let Some(alignment) = pixel_kind.unpack_alignment() {
                temp_binding
                    .server
                    .gl
                    .pixel_store_i32(glow::UNPACK_ALIGNMENT, alignment);
            }

            if let Some(swizzle_mask) = swizzle_mask {
                if temp_binding
                    .server
                    .gl
                    .supported_extensions()
                    .contains("GL_ARB_texture_swizzle")
                {
                    temp_binding.server.gl.tex_parameter_i32_slice(
                        target,
                        glow::TEXTURE_SWIZZLE_RGBA,
                        &swizzle_mask,
                    );
                }
            }

            let mut mip_byte_offset = 0;
            'mip_loop2: for mip in 0..mip_count {
                match kind {
                    GpuTextureKind::Line { length } => {
                        if let Some(length) = length.checked_shr(mip as u32) {
                            let size = image_1d_size_bytes(pixel_kind, length) as i32;
                            let pixels = data.map(|data| {
                                &data[mip_byte_offset..(mip_byte_offset + size as usize)]
                            });

                            if is_compressed {
                                temp_binding.server.gl.compressed_tex_image_1d(
                                    glow::TEXTURE_1D,
                                    mip as i32,
                                    internal_format as i32,
                                    length as i32,
                                    0,
                                    size,
                                    pixels.ok_or(FrameworkError::EmptyTextureData)?,
                                );
                            } else {
                                temp_binding.server.gl.tex_image_1d(
                                    glow::TEXTURE_1D,
                                    mip as i32,
                                    internal_format as i32,
                                    length as i32,
                                    0,
                                    format,
                                    data_type,
                                    pixels,
                                );
                            }

                            mip_byte_offset += size as usize;
                        } else {
                            // No need to add degenerated mips (0x1, 0x2, 4x0, etc).
                            break 'mip_loop2;
                        }
                    }
                    GpuTextureKind::Rectangle { width, height } => {
                        if let (Some(width), Some(height)) = (
                            width.checked_shr(mip as u32),
                            height.checked_shr(mip as u32),
                        ) {
                            let size = image_2d_size_bytes(pixel_kind, width, height) as i32;
                            let pixels = data.map(|data| {
                                &data[mip_byte_offset..(mip_byte_offset + size as usize)]
                            });

                            if is_compressed {
                                temp_binding.server.gl.compressed_tex_image_2d(
                                    glow::TEXTURE_2D,
                                    mip as i32,
                                    internal_format as i32,
                                    width as i32,
                                    height as i32,
                                    0,
                                    size,
                                    pixels.ok_or(FrameworkError::EmptyTextureData)?,
                                );
                            } else {
                                temp_binding.server.gl.tex_image_2d(
                                    glow::TEXTURE_2D,
                                    mip as i32,
                                    internal_format as i32,
                                    width as i32,
                                    height as i32,
                                    0,
                                    format,
                                    data_type,
                                    pixels,
                                );
                            }

                            mip_byte_offset += size as usize;
                        } else {
                            // No need to add degenerated mips (0x1, 0x2, 4x0, etc).
                            break 'mip_loop2;
                        }
                    }
                    GpuTextureKind::Cube { width, height } => {
                        if let (Some(width), Some(height)) = (
                            width.checked_shr(mip as u32),
                            height.checked_shr(mip as u32),
                        ) {
                            let bytes_per_face = image_2d_size_bytes(pixel_kind, width, height);

                            for face in 0..6 {
                                let begin = mip_byte_offset + face * bytes_per_face;
                                let end = mip_byte_offset + (face + 1) * bytes_per_face;
                                let face_pixels = data.map(|data| &data[begin..end]);

                                if is_compressed {
                                    temp_binding.server.gl.compressed_tex_image_2d(
                                        glow::TEXTURE_CUBE_MAP_POSITIVE_X + face as u32,
                                        mip as i32,
                                        internal_format as i32,
                                        width as i32,
                                        height as i32,
                                        0,
                                        bytes_per_face as i32,
                                        face_pixels.ok_or(FrameworkError::EmptyTextureData)?,
                                    );
                                } else {
                                    temp_binding.server.gl.tex_image_2d(
                                        glow::TEXTURE_CUBE_MAP_POSITIVE_X + face as u32,
                                        mip as i32,
                                        internal_format as i32,
                                        width as i32,
                                        height as i32,
                                        0,
                                        format,
                                        data_type,
                                        face_pixels,
                                    );
                                }
                            }

                            mip_byte_offset += 6 * bytes_per_face;
                        } else {
                            // No need to add degenerated mips (0x1, 0x2, 4x0, etc).
                            break 'mip_loop2;
                        }
                    }
                    GpuTextureKind::Volume {
                        width,
                        height,
                        depth,
                    } => {
                        if let (Some(width), Some(height), Some(depth)) = (
                            width.checked_shr(mip as u32),
                            height.checked_shr(mip as u32),
                            depth.checked_shr(mip as u32),
                        ) {
                            let size = image_3d_size_bytes(pixel_kind, width, height, depth) as i32;
                            let pixels = data.map(|data| {
                                &data[mip_byte_offset..(mip_byte_offset + size as usize)]
                            });

                            if is_compressed {
                                temp_binding.server.gl.compressed_tex_image_3d(
                                    glow::TEXTURE_3D,
                                    mip as i32,
                                    internal_format as i32,
                                    width as i32,
                                    height as i32,
                                    depth as i32,
                                    0,
                                    size,
                                    pixels.ok_or(FrameworkError::EmptyTextureData)?,
                                );
                            } else {
                                temp_binding.server.gl.tex_image_3d(
                                    glow::TEXTURE_3D,
                                    mip as i32,
                                    internal_format as i32,
                                    width as i32,
                                    height as i32,
                                    depth as i32,
                                    0,
                                    format,
                                    data_type,
                                    pixels,
                                );
                            }

                            mip_byte_offset += size as usize;
                        } else {
                            // No need to add degenerated mips (0x1, 0x2, 4x0, etc).
                            break 'mip_loop2;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn read_pixels(&self) -> Vec<u8> {
        let temp_binding = self.make_temp_binding();
        unsafe {
            if let GpuTextureKind::Rectangle { width, height } = self.kind {
                let pixel_info = self.pixel_kind.pixel_descriptor();
                let mut buffer = vec![0; image_2d_size_bytes(self.pixel_kind, width, height)];
                temp_binding.server.gl.read_pixels(
                    0,
                    0,
                    width as i32,
                    height as i32,
                    pixel_info.format,
                    pixel_info.data_type,
                    PixelPackData::Slice(buffer.as_mut_slice()),
                );
                buffer
            } else {
                Default::default()
            }
        }
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

    pub fn get_image<T: Pod>(&self, level: usize) -> Vec<T> {
        let temp_binding = self.make_temp_binding();
        unsafe {
            let desc = self.pixel_kind.pixel_descriptor();
            let (kind, buffer_size) = match self.kind {
                GpuTextureKind::Line { length } => (
                    glow::TEXTURE_1D,
                    image_1d_size_bytes(self.pixel_kind, length),
                ),
                GpuTextureKind::Rectangle { width, height } => (
                    glow::TEXTURE_2D,
                    image_2d_size_bytes(self.pixel_kind, width, height),
                ),
                GpuTextureKind::Cube { width, height } => (
                    glow::TEXTURE_CUBE_MAP,
                    6 * image_2d_size_bytes(self.pixel_kind, width, height),
                ),
                GpuTextureKind::Volume {
                    width,
                    height,
                    depth,
                } => (
                    glow::TEXTURE_3D,
                    image_3d_size_bytes(self.pixel_kind, width, height, depth),
                ),
            };

            let mut bytes = vec![0; buffer_size];
            temp_binding.server.gl.get_tex_image(
                kind,
                level as i32,
                desc.format,
                desc.data_type,
                PixelPackData::Slice(bytes.as_mut_slice()),
            );
            let typed = Vec::<T>::from_raw_parts(
                bytes.as_mut_ptr() as *mut T,
                bytes.len() / size_of::<T>(),
                bytes.capacity() / size_of::<T>(),
            );

            std::mem::forget(bytes);
            typed
        }
    }

    pub fn kind(&self) -> GpuTextureKind {
        self.kind
    }

    pub fn id(&self) -> glow::Texture {
        self.texture
    }

    pub fn minification_filter(&self) -> MinificationFilter {
        self.min_filter
    }

    pub fn magnification_filter(&self) -> MagnificationFilter {
        self.mag_filter
    }

    pub fn s_wrap_mode(&self) -> WrapMode {
        self.s_wrap_mode
    }

    pub fn t_wrap_mode(&self) -> WrapMode {
        self.t_wrap_mode
    }

    pub fn anisotropy(&self) -> f32 {
        self.anisotropy
    }

    pub fn pixel_kind(&self) -> PixelKind {
        self.pixel_kind
    }
}

impl Drop for GpuTexture {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            unsafe {
                state.gl.delete_texture(self.texture);
            }
        }
    }
}
