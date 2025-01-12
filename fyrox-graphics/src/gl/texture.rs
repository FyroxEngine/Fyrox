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
    core::color::Color,
    error::FrameworkError,
    gl::{server::GlGraphicsServer, ToGlConstant},
    gpu_texture::{
        image_1d_size_bytes, image_2d_size_bytes, image_3d_size_bytes, Coordinate, CubeMapFace,
        GpuTexture, GpuTextureDescriptor, GpuTextureKind, MagnificationFilter, MinificationFilter,
        PixelKind, WrapMode,
    },
};
use glow::{HasContext, PixelPackData, COMPRESSED_RED_RGTC1, COMPRESSED_RG_RGTC2};
use std::{
    marker::PhantomData,
    rc::{Rc, Weak},
};

impl GpuTextureKind {
    pub fn gl_texture_target(&self) -> u32 {
        match self {
            Self::Line { .. } => glow::TEXTURE_1D,
            Self::Rectangle { .. } => glow::TEXTURE_2D,
            Self::Cube { .. } => glow::TEXTURE_CUBE_MAP,
            Self::Volume { .. } => glow::TEXTURE_3D,
        }
    }
}

impl ToGlConstant for MinificationFilter {
    fn into_gl(self) -> u32 {
        match self {
            Self::Nearest => glow::NEAREST,
            Self::NearestMipMapNearest => glow::NEAREST_MIPMAP_NEAREST,
            Self::NearestMipMapLinear => glow::NEAREST_MIPMAP_LINEAR,
            Self::Linear => glow::LINEAR,
            Self::LinearMipMapNearest => glow::LINEAR_MIPMAP_NEAREST,
            Self::LinearMipMapLinear => glow::LINEAR_MIPMAP_LINEAR,
        }
    }
}

impl ToGlConstant for MagnificationFilter {
    fn into_gl(self) -> u32 {
        match self {
            Self::Nearest => glow::NEAREST,
            Self::Linear => glow::LINEAR,
        }
    }
}

impl ToGlConstant for WrapMode {
    fn into_gl(self) -> u32 {
        match self {
            Self::Repeat => glow::REPEAT,
            Self::ClampToEdge => glow::CLAMP_TO_EDGE,
            Self::ClampToBorder => glow::CLAMP_TO_BORDER,
            Self::MirroredRepeat => glow::MIRRORED_REPEAT,
            Self::MirrorClampToEdge => glow::MIRROR_CLAMP_TO_EDGE,
        }
    }
}

impl ToGlConstant for Coordinate {
    fn into_gl(self) -> u32 {
        match self {
            Self::S => glow::TEXTURE_WRAP_S,
            Self::T => glow::TEXTURE_WRAP_T,
            Self::R => glow::TEXTURE_WRAP_R,
        }
    }
}

impl ToGlConstant for CubeMapFace {
    fn into_gl(self) -> u32 {
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

pub struct GlTexture {
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
    base_level: usize,
    max_level: usize,
    min_lod: f32,
    max_lod: f32,
    lod_bias: f32,
    // Force compiler to not implement Send and Sync, because OpenGL is not thread-safe.
    thread_mark: PhantomData<*const u8>,
}

const GL_COMPRESSED_RGB_S3TC_DXT1_EXT: u32 = 0x83F0;
const GL_COMPRESSED_RGBA_S3TC_DXT1_EXT: u32 = 0x83F1;
const GL_COMPRESSED_RGBA_S3TC_DXT3_EXT: u32 = 0x83F2;
const GL_COMPRESSED_RGBA_S3TC_DXT5_EXT: u32 = 0x83F3;

pub struct PixelDescriptor {
    pub data_type: u32,
    pub format: u32,
    pub internal_format: u32,
    pub swizzle_mask: Option<[i32; 4]>,
}

impl PixelKind {
    pub(crate) fn pixel_descriptor(self) -> PixelDescriptor {
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

struct TempBinding {
    server: Rc<GlGraphicsServer>,
    unit: u32,
    target: u32,
}

impl TempBinding {
    fn new(server: Rc<GlGraphicsServer>, texture: &GlTexture) -> Self {
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

    fn set_minification_filter(&mut self, min_filter: MinificationFilter) {
        unsafe {
            self.server.gl.tex_parameter_i32(
                self.target,
                glow::TEXTURE_MIN_FILTER,
                min_filter.into_gl() as i32,
            );
        }
    }

    fn set_magnification_filter(&mut self, mag_filter: MagnificationFilter) {
        unsafe {
            self.server.gl.tex_parameter_i32(
                self.target,
                glow::TEXTURE_MAG_FILTER,
                mag_filter.into_gl() as i32,
            );
        }
    }

    fn set_anisotropy(&mut self, anisotropy: f32) {
        unsafe {
            let max = self
                .server
                .gl
                .get_parameter_f32(glow::MAX_TEXTURE_MAX_ANISOTROPY_EXT);
            self.server.gl.tex_parameter_f32(
                self.target,
                glow::TEXTURE_MAX_ANISOTROPY_EXT,
                anisotropy.clamp(1.0, max),
            );
        }
    }

    fn set_wrap(&mut self, coordinate: Coordinate, wrap: WrapMode) {
        unsafe {
            self.server.gl.tex_parameter_i32(
                self.target,
                coordinate.into_gl(),
                wrap.into_gl() as i32,
            );
        }
    }

    fn set_base_level(&mut self, level: usize) {
        unsafe {
            self.server
                .gl
                .tex_parameter_i32(self.target, glow::TEXTURE_BASE_LEVEL, level as i32);
        }
    }

    fn set_max_level(&mut self, level: usize) {
        unsafe {
            self.server
                .gl
                .tex_parameter_i32(self.target, glow::TEXTURE_MAX_LEVEL, level as i32);
        }
    }

    fn set_min_lod(&mut self, min_lod: f32) {
        unsafe {
            self.server
                .gl
                .tex_parameter_f32(self.target, glow::TEXTURE_MIN_LOD, min_lod);
        }
    }

    fn set_max_lod(&mut self, max_lod: f32) {
        unsafe {
            self.server
                .gl
                .tex_parameter_f32(self.target, glow::TEXTURE_MAX_LOD, max_lod);
        }
    }

    fn set_lod_bias(&mut self, bias: f32) {
        unsafe {
            self.server
                .gl
                .tex_parameter_f32(self.target, glow::TEXTURE_LOD_BIAS, bias);
        }
    }
}

impl Drop for TempBinding {
    fn drop(&mut self) {
        self.server.set_texture(self.unit, self.target, None);
    }
}

impl GlTexture {
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
        mut desc: GpuTextureDescriptor,
    ) -> Result<Self, FrameworkError> {
        // Clamp the mip level values to sensible range to prevent weird behavior.
        let actual_max_level = desc.mip_count.saturating_sub(1);
        if desc.max_level > actual_max_level {
            desc.max_level = actual_max_level;
        }
        if desc.base_level > desc.max_level {
            desc.base_level = desc.max_level;
        }
        if desc.base_level > actual_max_level {
            desc.base_level = actual_max_level;
        }

        unsafe {
            let texture = server.gl.create_texture()?;

            let mut result = Self {
                state: server.weak(),
                texture,
                kind: desc.kind,
                min_filter: desc.min_filter,
                mag_filter: desc.mag_filter,
                s_wrap_mode: desc.s_wrap_mode,
                t_wrap_mode: desc.t_wrap_mode,
                r_wrap_mode: desc.r_wrap_mode,
                anisotropy: desc.anisotropy,
                pixel_kind: desc.pixel_kind,
                base_level: desc.base_level,
                max_level: desc.max_level,
                min_lod: desc.min_lod,
                max_lod: desc.max_lod,
                lod_bias: desc.lod_bias,
                thread_mark: PhantomData,
            };

            result.set_data(desc.kind, desc.pixel_kind, desc.mip_count, desc.data)?;

            let mut binding = result.make_temp_binding();
            binding.set_magnification_filter(desc.mag_filter);
            binding.set_minification_filter(desc.min_filter);
            binding.set_wrap(Coordinate::S, desc.s_wrap_mode);
            binding.set_wrap(Coordinate::T, desc.t_wrap_mode);
            binding.set_wrap(Coordinate::R, desc.r_wrap_mode);
            binding.set_anisotropy(desc.anisotropy);
            binding.set_base_level(desc.base_level);
            binding.set_max_level(desc.max_level);
            binding.set_min_lod(desc.min_lod);
            binding.set_max_lod(desc.max_lod);
            binding.set_lod_bias(desc.lod_bias);

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

    pub fn id(&self) -> glow::Texture {
        self.texture
    }
}

impl Drop for GlTexture {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            unsafe {
                state.gl.delete_texture(self.texture);
            }
        }
    }
}

impl GpuTexture for GlTexture {
    fn set_anisotropy(&mut self, anisotropy: f32) {
        self.make_temp_binding().set_anisotropy(anisotropy);
        self.anisotropy = anisotropy;
    }

    fn anisotropy(&self) -> f32 {
        self.anisotropy
    }

    fn set_minification_filter(&mut self, filter: MinificationFilter) {
        self.make_temp_binding().set_minification_filter(filter);
        self.min_filter = filter;
    }

    fn minification_filter(&self) -> MinificationFilter {
        self.min_filter
    }

    fn set_magnification_filter(&mut self, filter: MagnificationFilter) {
        self.make_temp_binding().set_magnification_filter(filter);
        self.mag_filter = filter;
    }

    fn magnification_filter(&self) -> MagnificationFilter {
        self.mag_filter
    }

    fn set_wrap(&mut self, coordinate: Coordinate, wrap: WrapMode) {
        self.make_temp_binding().set_wrap(coordinate, wrap);
        match coordinate {
            Coordinate::S => self.s_wrap_mode = wrap,
            Coordinate::T => self.t_wrap_mode = wrap,
            Coordinate::R => self.r_wrap_mode = wrap,
        }
    }

    fn wrap_mode(&self, coordinate: Coordinate) -> WrapMode {
        match coordinate {
            Coordinate::S => self.s_wrap_mode,
            Coordinate::T => self.t_wrap_mode,
            Coordinate::R => self.r_wrap_mode,
        }
    }

    fn set_border_color(&mut self, #[allow(unused_variables)] color: Color) {
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

    fn set_data(
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

        let mut temp_binding = self.make_temp_binding();
        temp_binding.set_max_level(mip_count.saturating_sub(1));
        let target = kind.gl_texture_target();

        unsafe {
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

    fn get_image(&self, level: usize) -> Vec<u8> {
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
            bytes
        }
    }

    fn read_pixels(&self) -> Vec<u8> {
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

    fn kind(&self) -> GpuTextureKind {
        self.kind
    }

    fn pixel_kind(&self) -> PixelKind {
        self.pixel_kind
    }

    fn set_base_level(&mut self, level: usize) {
        self.make_temp_binding().set_base_level(level);
        self.base_level = level;
    }

    fn base_level(&self) -> usize {
        self.base_level
    }

    fn set_max_level(&mut self, level: usize) {
        self.make_temp_binding().set_max_level(level);
        self.max_level = level;
    }

    fn max_level(&self) -> usize {
        self.max_level
    }

    fn set_min_lod(&mut self, min_lod: f32) {
        self.make_temp_binding().set_min_lod(min_lod);
        self.min_lod = min_lod;
    }

    fn min_lod(&self) -> f32 {
        self.min_lod
    }

    fn set_max_lod(&mut self, max_lod: f32) {
        self.make_temp_binding().set_max_lod(max_lod);
        self.max_lod = max_lod;
    }

    fn max_lod(&self) -> f32 {
        self.max_lod
    }

    fn set_lod_bias(&mut self, bias: f32) {
        self.make_temp_binding().set_lod_bias(bias);
        self.lod_bias = bias;
    }

    fn lod_bias(&self) -> f32 {
        self.lod_bias
    }
}
