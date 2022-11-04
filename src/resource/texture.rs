//! Texture is an image that used to fill faces to add details to them.
//!
//! In most cases textures are just 2D images, however there are some exclusions to that -
//! for example cube maps, that may be used for environment mapping. Fyrox supports 1D, 2D,
//! 3D and Cube textures.
//!
//! ## Supported formats
//!
//! To load images and decode them, Fyrox uses image and ddsfile crates. Here is the list of
//! supported formats: png, tga, bmp, dds, jpg, gif, tiff, dds.
//!
//! ## Compressed textures
//!
//! Fyrox supports most commonly used formats of compressed textures: DXT1, DXT3, DXT5.
//!
//! ## Render target
//!
//! Texture can be used as render target to render scene in it. To do this you should use
//! new_render_target method and pass its result to scene's render target property. Renderer
//! will automatically provide you info about metrics of texture, but it won't give you
//! access to pixels of render target.

use crate::{
    asset::{define_new_resource, Resource, ResourceData, ResourceState},
    core::{
        futures::io::Error,
        io::{self, FileLoadError},
        reflect::prelude::*,
        visitor::{PodVecView, Visit, VisitError, VisitResult, Visitor},
    },
    engine::resource_manager::options::ImportOptions,
};
use ddsfile::{Caps2, D3DFormat};
use fxhash::FxHasher;
use image::{imageops::FilterType, ColorType, DynamicImage, ImageError, ImageFormat};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::{
    borrow::Cow,
    fmt::{Debug, Formatter},
    hash::{Hash, Hasher},
    io::Cursor,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};
use strum_macros::{AsRefStr, EnumString, EnumVariantNames};

/// Texture kind.
#[derive(Copy, Clone, Debug)]
pub enum TextureKind {
    /// 1D texture.
    Line {
        /// Length of the texture.
        length: u32,
    },
    /// 2D texture.
    Rectangle {
        /// Width of the texture.
        width: u32,
        /// Height of the texture.
        height: u32,
    },
    /// Cube texture.
    Cube {
        /// Width of the cube face.
        width: u32,
        /// Height of the cube face.
        height: u32,
    },
    /// Volume texture (3D).
    Volume {
        /// Width of the volume.
        width: u32,
        /// Height of the volume.
        height: u32,
        /// Depth of the volume.
        depth: u32,
    },
}

impl Default for TextureKind {
    fn default() -> Self {
        Self::Rectangle {
            width: 0,
            height: 0,
        }
    }
}

impl Visit for TextureKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut id = match self {
            TextureKind::Line { .. } => 0,
            TextureKind::Rectangle { .. } => 1,
            TextureKind::Cube { .. } => 2,
            TextureKind::Volume { .. } => 3,
        };
        id.visit("Id", &mut region)?;
        if region.is_reading() {
            *self = match id {
                0 => TextureKind::Line { length: 0 },
                1 => TextureKind::Rectangle {
                    width: 0,
                    height: 0,
                },
                2 => TextureKind::Cube {
                    width: 0,
                    height: 0,
                },
                3 => TextureKind::Volume {
                    width: 0,
                    height: 0,
                    depth: 0,
                },
                _ => {
                    return VisitResult::Err(VisitError::User(format!(
                        "Invalid texture kind {}!",
                        id
                    )))
                }
            };
        }
        match self {
            TextureKind::Line { length } => {
                length.visit("Length", &mut region)?;
            }
            TextureKind::Rectangle { width, height } => {
                width.visit("Width", &mut region)?;
                height.visit("Height", &mut region)?;
            }
            TextureKind::Cube { width, height } => {
                width.visit("Width", &mut region)?;
                height.visit("Height", &mut region)?;
            }
            TextureKind::Volume {
                width,
                height,
                depth,
            } => {
                width.visit("Width", &mut region)?;
                height.visit("Height", &mut region)?;
                depth.visit("Depth", &mut region)?;
            }
        }

        Ok(())
    }
}

#[derive(Default, Clone)]
struct TextureBytes(Vec<u8>);

impl Visit for TextureBytes {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.0.visit(name, visitor)
    }
}

impl Debug for TextureBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Texture has {} bytes", self.0.len())
    }
}

impl From<Vec<u8>> for TextureBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl Deref for TextureBytes {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TextureBytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Actual texture data.
#[derive(Debug)]
pub struct TextureData {
    path: PathBuf,
    kind: TextureKind,
    bytes: TextureBytes,
    pixel_kind: TexturePixelKind,
    minification_filter: TextureMinificationFilter,
    magnification_filter: TextureMagnificationFilter,
    s_wrap_mode: TextureWrapMode,
    t_wrap_mode: TextureWrapMode,
    mip_count: u32,
    anisotropy: f32,
    serialize_content: bool,
    data_hash: u64,
    is_render_target: bool,
}

impl ResourceData for TextureData {
    fn path(&self) -> Cow<Path> {
        Cow::Borrowed(&self.path)
    }

    fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }
}

impl Visit for TextureData {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        let mut kind = self.pixel_kind.id();
        kind.visit("KindId", &mut region)?;
        if region.is_reading() {
            self.pixel_kind = TexturePixelKind::new(kind)?;
        }

        self.path.visit("Path", &mut region)?;

        self.minification_filter
            .visit("MinificationFilter", &mut region)?;
        self.magnification_filter
            .visit("MagnificationFilter", &mut region)?;
        self.anisotropy.visit("Anisotropy", &mut region)?;
        self.s_wrap_mode.visit("SWrapMode", &mut region)?;
        self.t_wrap_mode.visit("TWrapMode", &mut region)?;
        self.mip_count.visit("MipCount", &mut region)?;
        self.kind.visit("Kind", &mut region)?;
        let _ = self
            .serialize_content
            .visit("SerializeContent", &mut region);

        if self.serialize_content {
            let mut bytes_view = PodVecView::from_pod_vec(&mut self.bytes);
            bytes_view.visit("Data", &mut region)?;
        }

        Ok(())
    }
}

impl Default for TextureData {
    /// It is very important to mention that defaults may be different for texture when you
    /// importing them through resource manager, see
    /// [TextureImportOptions](../engine/resource_manager/struct.TextureImportOptions.html) for more info.
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            kind: TextureKind::Rectangle {
                width: 0,
                height: 0,
            },
            bytes: Default::default(),
            pixel_kind: TexturePixelKind::RGBA8,
            minification_filter: TextureMinificationFilter::LinearMipMapLinear,
            magnification_filter: TextureMagnificationFilter::Linear,
            s_wrap_mode: TextureWrapMode::Repeat,
            t_wrap_mode: TextureWrapMode::Repeat,
            mip_count: 1,
            anisotropy: 16.0,
            serialize_content: false,
            data_hash: 0,
            is_render_target: false,
        }
    }
}

/// Allows you to define a set of parameters for a texture resource.
///
/// # Details
///
/// Usually the content of this structure is stored in a separate file with .options extension. Typical content of
/// a settings file should look like this:
///
/// ```text
/// (
///     minification_filter: Linear,
///     magnification_filter: Linear,
///     s_wrap_mode: Repeat,
///     t_wrap_mode: ClampToEdge,
///     anisotropy: 8.0,
///     compression: NoCompression,    
/// )
/// ```
#[derive(Clone, Deserialize, Serialize, Reflect)]
pub struct TextureImportOptions {
    #[serde(default)]
    pub(crate) minification_filter: TextureMinificationFilter,
    #[serde(default)]
    pub(crate) magnification_filter: TextureMagnificationFilter,
    #[serde(default)]
    pub(crate) s_wrap_mode: TextureWrapMode,
    #[serde(default)]
    pub(crate) t_wrap_mode: TextureWrapMode,
    #[serde(default)]
    pub(crate) anisotropy: f32,
    #[serde(default)]
    pub(crate) compression: CompressionOptions,
}

impl Default for TextureImportOptions {
    fn default() -> Self {
        Self {
            minification_filter: TextureMinificationFilter::LinearMipMapLinear,
            magnification_filter: TextureMagnificationFilter::Linear,
            s_wrap_mode: TextureWrapMode::Repeat,
            t_wrap_mode: TextureWrapMode::Repeat,
            anisotropy: 16.0,
            compression: CompressionOptions::default(),
        }
    }
}

impl ImportOptions for TextureImportOptions {}

impl TextureImportOptions {
    /// Sets new minification filter which will be applied to every imported texture as
    /// default value.
    pub fn with_minification_filter(
        mut self,
        minification_filter: TextureMinificationFilter,
    ) -> Self {
        self.minification_filter = minification_filter;
        self
    }

    /// Sets new minification filter which will be applied to every imported texture as
    /// default value.
    pub fn set_minification_filter(&mut self, minification_filter: TextureMinificationFilter) {
        self.minification_filter = minification_filter;
    }

    /// Sets new magnification filter which will be applied to every imported texture as
    /// default value.
    pub fn with_magnification_filter(
        mut self,
        magnification_filter: TextureMagnificationFilter,
    ) -> Self {
        self.magnification_filter = magnification_filter;
        self
    }

    /// Sets new magnification filter which will be applied to every imported texture as
    /// default value.
    pub fn set_magnification_filter(&mut self, magnification_filter: TextureMagnificationFilter) {
        self.magnification_filter = magnification_filter;
    }

    /// Sets new S coordinate wrap mode which will be applied to every imported texture as
    /// default value.
    pub fn with_s_wrap_mode(mut self, s_wrap_mode: TextureWrapMode) -> Self {
        self.s_wrap_mode = s_wrap_mode;
        self
    }

    /// Sets new S coordinate wrap mode which will be applied to every imported texture as
    /// default value.
    pub fn set_s_wrap_mode(&mut self, s_wrap_mode: TextureWrapMode) {
        self.s_wrap_mode = s_wrap_mode;
    }

    /// Sets new T coordinate wrap mode which will be applied to every imported texture as
    /// default value.
    pub fn with_t_wrap_mode(mut self, t_wrap_mode: TextureWrapMode) -> Self {
        self.t_wrap_mode = t_wrap_mode;
        self
    }

    /// Sets new T coordinate wrap mode which will be applied to every imported texture as
    /// default value.
    pub fn set_t_wrap_mode(&mut self, t_wrap_mode: TextureWrapMode) {
        self.t_wrap_mode = t_wrap_mode;
    }

    /// Sets new anisotropy level which will be applied to every imported texture as
    /// default value.
    pub fn with_anisotropy(mut self, anisotropy: f32) -> Self {
        self.anisotropy = anisotropy.min(1.0);
        self
    }

    /// Sets new anisotropy level which will be applied to every imported texture as
    /// default value.
    pub fn set_anisotropy(&mut self, anisotropy: f32) {
        self.anisotropy = anisotropy.min(1.0);
    }

    /// Sets desired texture compression.
    pub fn with_compression(mut self, compression: CompressionOptions) -> Self {
        self.compression = compression;
        self
    }

    /// Sets desired texture compression.
    pub fn set_compression(&mut self, compression: CompressionOptions) {
        self.compression = compression;
    }
}

define_new_resource!(
    /// See module docs.
    #[derive(Reflect)]
    #[reflect(hide_all)]
    Texture<TextureData, TextureError>
);

/// Texture state alias.
pub type TextureState = ResourceState<TextureData, TextureError>;

impl Texture {
    /// Creates new render target for a scene. This method automatically configures GPU texture
    /// to correct settings, after render target was created, it must not be modified, otherwise
    /// result is undefined.
    pub fn new_render_target(width: u32, height: u32) -> Self {
        Self(Resource::new(TextureState::Ok(TextureData {
            path: Default::default(),
            // Render target will automatically set width and height before rendering.
            kind: TextureKind::Rectangle { width, height },
            bytes: Default::default(),
            pixel_kind: TexturePixelKind::RGBA8,
            minification_filter: TextureMinificationFilter::Linear,
            magnification_filter: TextureMagnificationFilter::Linear,
            s_wrap_mode: TextureWrapMode::Repeat,
            t_wrap_mode: TextureWrapMode::Repeat,
            mip_count: 1,
            anisotropy: 1.0,
            serialize_content: false,
            data_hash: 0,
            is_render_target: true,
        })))
    }

    /// Tries to load a texture from given data. Use this method if you want to
    /// load a texture from embedded data.
    ///
    /// # On-demand compression
    ///
    /// The data can be compressed if needed to improve performance on GPU side.
    ///
    /// # Important notes
    ///
    /// Textures loaded with this method won't be correctly serialized! It means
    /// that if you'll made a scene with textures loaded with this method, and then
    /// save a scene, then the engine won't be able to restore the textures if you'll
    /// try to load the saved scene. This is essential limitation of this method,
    /// because the engine does not know where to get the data of the texture at
    /// loading. You should use `ResourceManager::request_texture` in majority of cases!
    ///
    /// Main use cases for this method are: procedural textures, icons for GUI.
    pub fn load_from_memory(
        data: &[u8],
        compression: CompressionOptions,
        gen_mip_maps: bool,
    ) -> Result<Self, TextureError> {
        Ok(Self(Resource::new(TextureState::Ok(
            TextureData::load_from_memory(data, compression, gen_mip_maps)?,
        ))))
    }

    /// Tries to create new texture from given parameters, it may fail only if size of data passed
    /// in does not match with required.
    pub fn from_bytes(
        kind: TextureKind,
        pixel_kind: TexturePixelKind,
        bytes: Vec<u8>,
        serialize_content: bool,
    ) -> Option<Self> {
        Some(Self(Resource::new(TextureState::Ok(
            TextureData::from_bytes(kind, pixel_kind, bytes, serialize_content)?,
        ))))
    }
}

/// The texture magnification function is used when the pixel being textured maps to an area
/// less than or equal to one texture element.
#[derive(
    Copy,
    Clone,
    Debug,
    Hash,
    PartialOrd,
    PartialEq,
    Deserialize,
    Serialize,
    Reflect,
    EnumVariantNames,
    EnumString,
    AsRefStr,
    Visit,
    Eq,
)]
#[repr(u32)]
pub enum TextureMagnificationFilter {
    /// Returns the value of the texture element that is nearest to the center of the pixel
    /// being textured.
    Nearest = 0,

    /// Returns the weighted average of the four texture elements that are closest to the
    /// center of the pixel being textured.
    Linear = 1,
}

impl Default for TextureMagnificationFilter {
    fn default() -> Self {
        Self::Linear
    }
}

/// The texture minifying function is used whenever the pixel being textured maps to an area
/// greater than one texture element.
#[derive(
    Copy,
    Clone,
    Debug,
    Hash,
    PartialOrd,
    PartialEq,
    Deserialize,
    Serialize,
    Reflect,
    EnumVariantNames,
    EnumString,
    AsRefStr,
    Visit,
    Eq,
)]
#[repr(u32)]
pub enum TextureMinificationFilter {
    /// Returns the value of the texture element that is nearest to the center of the pixel
    /// being textured.
    Nearest = 0,

    /// Chooses the mipmap that most closely matches the size of the pixel being textured and
    /// uses the Nearest criterion (the texture element nearest to the center of the pixel)
    /// to produce a texture value.
    NearestMipMapNearest = 1,

    /// Chooses the two mipmaps that most closely match the size of the pixel being textured
    /// and uses the Nearest criterion (the texture element nearest to the center of the pixel)
    /// to produce a texture value from each mipmap. The final texture value is a weighted average
    /// of those two values.
    NearestMipMapLinear = 2,

    /// Returns the weighted average of the four texture elements that are closest to the
    /// center of the pixel being textured.
    Linear = 3,

    /// Chooses the mipmap that most closely matches the size of the pixel being textured and
    /// uses the Linear criterion (a weighted average of the four texture elements that are
    /// closest to the center of the pixel) to produce a texture value.
    LinearMipMapNearest = 4,

    /// Chooses the two mipmaps that most closely match the size of the pixel being textured
    /// and uses the Linear criterion (a weighted average of the four texture elements that
    /// are closest to the center of the pixel) to produce a texture value from each mipmap.
    /// The final texture value is a weighted average of those two values.
    LinearMipMapLinear = 5,
}

impl TextureMinificationFilter {
    /// Returns true if minification filter is using mip mapping, false - otherwise.
    pub fn is_using_mip_mapping(self) -> bool {
        match self {
            TextureMinificationFilter::Nearest | TextureMinificationFilter::Linear => false,
            TextureMinificationFilter::NearestMipMapNearest
            | TextureMinificationFilter::LinearMipMapLinear
            | TextureMinificationFilter::NearestMipMapLinear
            | TextureMinificationFilter::LinearMipMapNearest => true,
        }
    }
}

impl Default for TextureMinificationFilter {
    fn default() -> Self {
        Self::LinearMipMapLinear
    }
}

/// Defines a law of texture coordinate modification.
#[derive(
    Copy,
    Clone,
    Debug,
    Hash,
    PartialOrd,
    PartialEq,
    Deserialize,
    Serialize,
    Reflect,
    EnumVariantNames,
    EnumString,
    AsRefStr,
    Visit,
    Eq,
)]
#[repr(u32)]
pub enum TextureWrapMode {
    /// Causes the integer part of a coordinate to be ignored; GPU uses only the fractional part,
    /// thereby creating a repeating pattern.
    Repeat = 0,

    /// Causes a coordinates to be clamped to the range range, where N is the size of the texture
    /// in the direction of clamping
    ClampToEdge = 1,

    /// Evaluates a coordinates in a similar manner to ClampToEdge. However, in cases where clamping
    /// would have occurred in ClampToEdge mode, the fetched texel data is substituted with the values
    /// specified by border color.
    ClampToBorder = 2,

    /// Causes the a coordinate to be set to the fractional part of the texture coordinate if the integer
    /// part of coordinate is even; if the integer part of coordinate is odd, then the coordinate texture
    /// coordinate is set to 1-frac, where frac represents the fractional part of coordinate.
    MirroredRepeat = 3,

    /// Causes a coordinate to be repeated as for MirroredRepeat for one repetition of the texture, at
    /// which point the coordinate to be clamped as in ClampToEdge.
    MirrorClampToEdge = 4,
}

impl Default for TextureWrapMode {
    fn default() -> Self {
        Self::Repeat
    }
}

/// Texture kind defines pixel format of texture.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum TexturePixelKind {
    /// 1 byte red.
    R8 = 0,

    /// Red, green, and blue components, each by 1 byte.
    RGB8 = 1,

    /// Red, green, blue, and alpha components, each by 1 byte.
    RGBA8 = 2,

    /// Red and green, each by 1 byte.
    RG8 = 3,

    /// 2 byte red.
    R16 = 4,

    /// Red and green, each by 2 byte.
    RG16 = 5,

    /// Blue, green, and red components, each by 1 byte.
    BGR8 = 6,

    /// Blue, green, red and alpha components, each by 1 byte.
    BGRA8 = 7,

    /// Red, green, and blue components, each by 2 byte.
    RGB16 = 8,

    /// Red, green, blue, and alpha components, each by 2 byte.
    RGBA16 = 9,

    /// Compressed S3TC DXT1 RGB (no alpha).
    DXT1RGB = 10,

    /// Compressed S3TC DXT1 RGBA.
    DXT1RGBA = 11,

    /// Compressed S3TC DXT3 RGBA.
    DXT3RGBA = 12,

    /// Compressed S3TC DXT5 RGBA.
    DXT5RGBA = 13,

    /// Compressed R8 texture (RGTC).
    R8RGTC = 14,

    /// Compressed RG8 texture (RGTC).
    RG8RGTC = 15,

    /// Floating-point RGB texture with 32bit depth.
    RGB32F = 16,

    /// Floating-point RGBA texture with 32bit depth.
    RGBA32F = 17,

    /// 1 byte luminance texture where pixels will have (L, L, L, 1.0) value on fetching.
    ///
    /// # Platform-specific
    ///
    /// - WebAssembly - not supported, the image will act like [`Self::R8`] format, which
    ///   will have (R, 0.0, 0.0, 1.0) pixels.
    Luminance8 = 18,

    /// 1 byte for luminance and 1 for alpha, where all pixels will have (L, L, L, A) value on fetching.
    ///
    /// # Platform-specific
    ///
    /// - WebAssembly - not supported, the image will act like [`Self::RG8`] format, which
    ///   will have (R, G, R, G) pixels.
    LuminanceAlpha8 = 19,

    /// 2 byte luminance texture where pixels will have (L, L, L, 1.0) value on fetching.
    ///
    /// # Platform-specific
    ///
    /// - WebAssembly - not supported, the image will act like [`Self::R8`] format, which
    ///   will have (R, 0.0, 0.0, 1.0) pixels.
    Luminance16 = 20,

    /// 2 byte for luminance and 2 for alpha, where all pixels will have (L, L, L, A) value on fetching.
    ///
    /// # Platform-specific
    ///
    /// - WebAssembly - not supported, the image will act like [`Self::RG16`] format, which
    ///   will have (R, G, R, G) pixels.
    LuminanceAlpha16 = 21,
}

impl TexturePixelKind {
    fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::R8),
            1 => Ok(Self::RGB8),
            2 => Ok(Self::RGBA8),
            3 => Ok(Self::RG8),
            4 => Ok(Self::R16),
            5 => Ok(Self::RG16),
            6 => Ok(Self::BGR8),
            7 => Ok(Self::BGRA8),
            8 => Ok(Self::RGB16),
            9 => Ok(Self::RGBA16),
            10 => Ok(Self::DXT1RGB),
            11 => Ok(Self::DXT1RGBA),
            12 => Ok(Self::DXT3RGBA),
            13 => Ok(Self::DXT5RGBA),
            14 => Ok(Self::R8RGTC),
            15 => Ok(Self::RG8RGTC),
            16 => Ok(Self::RGB32F),
            17 => Ok(Self::RGBA32F),
            18 => Ok(Self::Luminance8),
            19 => Ok(Self::LuminanceAlpha8),
            20 => Ok(Self::Luminance16),
            21 => Ok(Self::LuminanceAlpha16),
            _ => Err(format!("Invalid texture kind {}!", id)),
        }
    }

    fn id(self) -> u32 {
        self as u32
    }
}

/// An error that may occur during texture operations.
#[derive(Debug)]
pub enum TextureError {
    /// Format (pixel format, dimensions) is not supported.
    UnsupportedFormat,
    /// An io error.
    Io(std::io::Error),
    /// Internal image crate error.
    Image(image::ImageError),
    /// An error occurred during file loading.
    FileLoadError(FileLoadError),
}

impl Display for TextureError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureError::UnsupportedFormat => {
                write!(f, "Unsupported format!")
            }
            TextureError::Io(v) => {
                write!(f, "An i/o error has occurred: {v}")
            }
            TextureError::Image(v) => {
                write!(f, "Image loading error {v}")
            }
            TextureError::FileLoadError(v) => {
                write!(f, "A file load error has occurred {v:?}")
            }
        }
    }
}

impl From<FileLoadError> for TextureError {
    fn from(v: FileLoadError) -> Self {
        Self::FileLoadError(v)
    }
}

impl From<image::ImageError> for TextureError {
    fn from(v: ImageError) -> Self {
        Self::Image(v)
    }
}

impl From<std::io::Error> for TextureError {
    fn from(v: Error) -> Self {
        Self::Io(v)
    }
}

fn ceil_div_4(x: u32) -> u32 {
    (x + 3) / 4
}

/// Texture compression options.
///
/// # Notes
///
/// Try to avoid using compression for normal maps, normals maps usually has smooth
/// gradients, but compression algorithms used by Fyrox cannot preserve good quality
/// of such gradients.
#[derive(
    Copy,
    Clone,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    Debug,
    Reflect,
    EnumVariantNames,
    EnumString,
    AsRefStr,
)]
#[repr(u32)]
pub enum CompressionOptions {
    /// An image will be stored without compression if it is not already compressed.
    NoCompression = 0,

    /// An image will be encoded via DXT1 (BC1) compression with low quality if is not
    /// already compressed.
    /// Compression ratio is 1:8 (without alpha) or 1:6 (with 1-bit alpha).
    /// This option provides maximum speed by having lowest requirements of memory
    /// bandwidth.
    Speed = 1,

    /// An image will be encoded via DXT5 (BC5) compression with high quality if it is
    /// not already compressed.
    /// Compression ratio is 1:4 (including alpha)
    /// This option is faster than `NoCompression` speed by lower requirements of memory
    /// bandwidth.
    Quality = 2,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self::NoCompression
    }
}

fn transmute_slice<T>(bytes: &[u8]) -> &'_ [T] {
    // This is absolutely safe because `image` crate's Rgb8/Rgba8/etc. and `tbc`s Rgb8/Rgba8/etc.
    // have exactly the same memory layout.
    unsafe {
        std::slice::from_raw_parts(
            bytes.as_ptr() as *const T,
            bytes.len() / std::mem::size_of::<T>(),
        )
    }
}

fn compress_bc1<T: tbc::color::ColorRgba8>(bytes: &[u8], width: usize, height: usize) -> Vec<u8> {
    tbc::encode_image_bc1_conv_u8::<T>(transmute_slice::<T>(bytes), width, height)
}

fn compress_bc3<T: tbc::color::ColorRgba8>(bytes: &[u8], width: usize, height: usize) -> Vec<u8> {
    tbc::encode_image_bc3_conv_u8::<T>(transmute_slice::<T>(bytes), width, height)
}

fn compress_r8_bc4<T: tbc::color::ColorRed8>(bytes: &[u8], width: usize, height: usize) -> Vec<u8> {
    tbc::encode_image_bc4_r8_conv_u8::<T>(transmute_slice::<T>(bytes), width, height)
}

fn compress_rg8_bc4<T: tbc::color::ColorRedGreen8>(
    bytes: &[u8],
    width: usize,
    height: usize,
) -> Vec<u8> {
    tbc::encode_image_bc4_rg8_conv_u8::<T>(transmute_slice::<T>(bytes), width, height)
}

fn data_hash(data: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    data.hash(&mut hasher);
    hasher.finish()
}

fn try_compress(
    image: &DynamicImage,
    w: usize,
    h: usize,
    compression: CompressionOptions,
) -> Option<(Vec<u8>, TexturePixelKind)> {
    let bytes = image.as_bytes();
    match (image, compression) {
        (DynamicImage::ImageRgb8(_), CompressionOptions::Speed) => Some((
            compress_bc1::<tbc::color::Rgb8>(bytes, w, h),
            TexturePixelKind::DXT1RGB,
        )),
        (DynamicImage::ImageRgb8(_), CompressionOptions::Quality) => Some((
            compress_bc3::<tbc::color::Rgb8>(bytes, w, h),
            TexturePixelKind::DXT5RGBA,
        )),
        (DynamicImage::ImageRgba8(_), CompressionOptions::Speed) => Some((
            compress_bc1::<tbc::color::Rgba8>(bytes, w, h),
            TexturePixelKind::DXT1RGBA,
        )),
        (DynamicImage::ImageRgba8(_), CompressionOptions::Quality) => Some((
            compress_bc3::<tbc::color::Rgba8>(bytes, w, h),
            TexturePixelKind::DXT5RGBA,
        )),
        (DynamicImage::ImageLuma8(_), CompressionOptions::Speed)
        | (DynamicImage::ImageLuma8(_), CompressionOptions::Quality) => Some((
            compress_r8_bc4::<tbc::color::Red8>(bytes, w, h),
            TexturePixelKind::R8RGTC,
        )),
        (DynamicImage::ImageLumaA8(_), CompressionOptions::Speed)
        | (DynamicImage::ImageLumaA8(_), CompressionOptions::Quality) => Some((
            compress_rg8_bc4::<tbc::color::RedGreen8>(bytes, w, h),
            TexturePixelKind::RG8RGTC,
        )),
        _ => None,
    }
}

fn bytes_in_first_mip(kind: TextureKind, pixel_kind: TexturePixelKind) -> u32 {
    let pixel_count = match kind {
        TextureKind::Line { length } => length,
        TextureKind::Rectangle { width, height } => width * height,
        TextureKind::Cube { width, height } => 6 * width * height,
        TextureKind::Volume {
            width,
            height,
            depth,
        } => width * height * depth,
    };
    match pixel_kind {
        // Uncompressed formats.
        TexturePixelKind::R8 | TexturePixelKind::Luminance8 => pixel_count,
        TexturePixelKind::R16
        | TexturePixelKind::LuminanceAlpha8
        | TexturePixelKind::Luminance16
        | TexturePixelKind::RG8 => 2 * pixel_count,
        TexturePixelKind::RGB8 | TexturePixelKind::BGR8 => 3 * pixel_count,
        TexturePixelKind::RGBA8
        | TexturePixelKind::BGRA8
        | TexturePixelKind::RG16
        | TexturePixelKind::LuminanceAlpha16 => 4 * pixel_count,
        TexturePixelKind::RGB16 => 6 * pixel_count,
        TexturePixelKind::RGBA16 => 8 * pixel_count,
        TexturePixelKind::RGB32F => 12 * pixel_count,
        TexturePixelKind::RGBA32F => 16 * pixel_count,

        // Compressed formats.
        TexturePixelKind::DXT1RGB
        | TexturePixelKind::DXT1RGBA
        | TexturePixelKind::DXT3RGBA
        | TexturePixelKind::DXT5RGBA
        | TexturePixelKind::R8RGTC
        | TexturePixelKind::RG8RGTC => {
            let block_size = match pixel_kind {
                TexturePixelKind::DXT1RGB
                | TexturePixelKind::DXT1RGBA
                | TexturePixelKind::R8RGTC => 8,
                TexturePixelKind::DXT3RGBA
                | TexturePixelKind::DXT5RGBA
                | TexturePixelKind::RG8RGTC => 16,
                _ => unreachable!(),
            };
            match kind {
                TextureKind::Line { length } => ceil_div_4(length) * block_size,
                TextureKind::Rectangle { width, height } => {
                    ceil_div_4(width) * ceil_div_4(height) * block_size
                }
                TextureKind::Cube { width, height } => {
                    6 * ceil_div_4(width) * ceil_div_4(height) * block_size
                }
                TextureKind::Volume {
                    width,
                    height,
                    depth,
                } => ceil_div_4(width) * ceil_div_4(height) * ceil_div_4(depth) * block_size,
            }
        }
    }
}

impl TextureData {
    /// Tries to load a texture from given data in one of the following formats: PNG, BMP, TGA, JPG, DDS, GIF. Use
    /// this method if you want to load a texture from embedded data.
    ///
    /// # On-demand compression and mip-map generation
    ///
    /// The data can be compressed if needed to improve performance on GPU side. Mip-maps can be generated as well.
    /// **CAVEAT:** Compression and mip-map generation **won't** be taken into account in case of **DDS** textures,
    /// because DDS can already contain such data, you should generate mips and compress DDS textures manually using
    /// some offline tool like DirectXTexTool or similar.
    ///
    /// # Important notes
    ///
    /// Textures loaded with this method won't be correctly serialized! It means that if you'll made a scene with
    /// textures loaded with this method, and then save a scene, then the engine won't be able to restore the textures
    /// if you'll try to load the saved scene. This is essential limitation of this method, because the engine does
    /// not know where to get the data of the texture at loading. You should use `ResourceManager::request_texture`
    /// in majority of cases!
    ///
    /// # Use cases
    ///
    /// Main use cases for this method are: procedural textures, icons for GUI.
    pub fn load_from_memory(
        data: &[u8],
        compression: CompressionOptions,
        gen_mip_maps: bool,
    ) -> Result<Self, TextureError> {
        // DDS is special. It can contain various kinds of textures as well as textures with
        // various pixel formats.
        //
        // TODO: Add support for DXGI formats.
        if let Ok(dds) = ddsfile::Dds::read(&mut Cursor::new(data)) {
            let d3dformat = dds
                .get_d3d_format()
                .ok_or(TextureError::UnsupportedFormat)?;
            let mip_count = dds.get_num_mipmap_levels();
            let mut bytes = dds.data;

            // Try to use as much formats as possible.
            let pixel_kind = match d3dformat {
                D3DFormat::DXT1 => TexturePixelKind::DXT1RGBA,
                D3DFormat::DXT3 => TexturePixelKind::DXT3RGBA,
                D3DFormat::DXT5 => TexturePixelKind::DXT5RGBA,
                D3DFormat::L8 | D3DFormat::A8 => TexturePixelKind::R8,
                D3DFormat::L16 => TexturePixelKind::R16,
                D3DFormat::R8G8B8 => TexturePixelKind::RGB8,
                D3DFormat::A8L8 => TexturePixelKind::RG8,
                D3DFormat::A8R8G8B8 => {
                    // // ARGB8 -> RGBA8
                    // assert_eq!(bytes.len() % 4, 0);
                    // for chunk in bytes.chunks_exact_mut(4) {
                    //     let a = chunk[0];
                    //     let r = chunk[1];
                    //     let g = chunk[2];
                    //     let b = chunk[3];
                    //     chunk[0] = r;
                    //     chunk[1] = g;
                    //     chunk[2] = b;
                    //     chunk[3] = a;
                    // }
                    TexturePixelKind::RGBA8
                }
                D3DFormat::G16R16 => {
                    // GR16 -> RG16
                    assert_eq!(bytes.len() % 4, 0);
                    for chunk in bytes.chunks_exact_mut(4) {
                        // Red Hi + Lo bytes
                        let gh = chunk[0];
                        let gl = chunk[1];
                        // Green Hi + Lo bytes
                        let rh = chunk[2];
                        let rl = chunk[3];
                        // Swap
                        chunk[0] = rh;
                        chunk[1] = rl;
                        chunk[2] = gh;
                        chunk[3] = gl;
                    }
                    TexturePixelKind::RG16
                }
                _ => return Err(TextureError::UnsupportedFormat),
            };

            Ok(Self {
                pixel_kind,
                data_hash: data_hash(&bytes),
                minification_filter: TextureMinificationFilter::LinearMipMapLinear,
                magnification_filter: TextureMagnificationFilter::Linear,
                s_wrap_mode: TextureWrapMode::Repeat,
                t_wrap_mode: TextureWrapMode::Repeat,
                mip_count,
                bytes: bytes.into(),
                kind: if dds.header.caps2 & Caps2::CUBEMAP == Caps2::CUBEMAP {
                    TextureKind::Cube {
                        width: dds.header.width,
                        height: dds.header.height,
                    }
                } else if dds.header.caps2 & Caps2::VOLUME == Caps2::VOLUME {
                    TextureKind::Volume {
                        width: dds.header.width,
                        height: dds.header.height,
                        depth: dds.header.depth.unwrap(),
                    }
                } else {
                    TextureKind::Rectangle {
                        width: dds.header.width,
                        height: dds.header.height,
                    }
                },
                ..Default::default()
            })
        } else {
            // Commonly used formats are all rectangle textures.
            let dyn_img = image::load_from_memory(data)
                // Try to load as TGA, this is needed because TGA is badly designed format and does not
                // have an identifier in the beginning of the file (so called "magic") that allows quickly
                // check if the file is really contains expected data.
                .or_else(|_| image::load_from_memory_with_format(data, ImageFormat::Tga))?;

            let width = dyn_img.width();
            let height = dyn_img.height();

            let mut pixel_kind = match dyn_img {
                DynamicImage::ImageLuma8(_) => TexturePixelKind::Luminance8,
                DynamicImage::ImageLumaA8(_) => TexturePixelKind::LuminanceAlpha8,
                DynamicImage::ImageRgb8(_) => TexturePixelKind::RGB8,
                DynamicImage::ImageRgba8(_) => TexturePixelKind::RGBA8,
                DynamicImage::ImageLuma16(_) => TexturePixelKind::Luminance16,
                DynamicImage::ImageLumaA16(_) => TexturePixelKind::LuminanceAlpha16,
                DynamicImage::ImageRgb16(_) => TexturePixelKind::RGB16,
                DynamicImage::ImageRgba16(_) => TexturePixelKind::RGBA16,
                DynamicImage::ImageRgb32F(_) => TexturePixelKind::RGB32F,
                DynamicImage::ImageRgba32F(_) => TexturePixelKind::RGBA32F,
                _ => unreachable!(),
            };

            let mut mip_count = 0;
            let mut bytes = Vec::new();

            if gen_mip_maps {
                let mut level_width = width;
                let mut level_height = height;
                let mut current_level = dyn_img;

                while level_width != 0 && level_height != 0 {
                    if mip_count != 0 {
                        current_level = current_level.resize_exact(
                            level_width,
                            level_height,
                            FilterType::Lanczos3,
                        );
                    }

                    mip_count += 1;

                    if compression == CompressionOptions::NoCompression {
                        bytes.extend_from_slice(current_level.as_bytes())
                    } else if let Some((compressed_data, new_pixel_kind)) = try_compress(
                        &current_level,
                        level_width as usize,
                        level_height as usize,
                        compression,
                    ) {
                        pixel_kind = new_pixel_kind;
                        bytes.extend_from_slice(&compressed_data);
                    } else {
                        bytes.extend_from_slice(current_level.as_bytes())
                    }

                    level_width = level_width.checked_shr(1).unwrap_or_default();
                    level_height = level_height.checked_shr(1).unwrap_or_default();
                }
            } else {
                mip_count = 1;

                if compression == CompressionOptions::NoCompression {
                    bytes.extend_from_slice(dyn_img.as_bytes());
                } else if let Some((compressed_data, new_pixel_kind)) =
                    try_compress(&dyn_img, width as usize, height as usize, compression)
                {
                    pixel_kind = new_pixel_kind;
                    bytes.extend_from_slice(&compressed_data);
                } else {
                    bytes.extend_from_slice(dyn_img.as_bytes())
                }
            }

            Ok(Self {
                pixel_kind,
                kind: TextureKind::Rectangle { width, height },
                data_hash: data_hash(&bytes),
                bytes: bytes.into(),
                mip_count,
                ..Default::default()
            })
        }
    }

    /// Tries to load a texture from a file.
    ///
    /// # Notes
    ///
    /// It is **not** public because you must use resource manager to load textures from external
    /// resources.
    pub(crate) async fn load_from_file<P: AsRef<Path>>(
        path: P,
        compression: CompressionOptions,
        gen_mip_maps: bool,
    ) -> Result<Self, TextureError> {
        let data = io::load_file(path.as_ref()).await?;
        let mut texture = Self::load_from_memory(&data, compression, gen_mip_maps)?;
        texture.path = path.as_ref().to_path_buf();
        Ok(texture)
    }

    /// Creates new texture instance from given parameters.
    ///
    /// # Limitations
    ///
    /// Currently textures with only one mip level are supported!
    pub fn from_bytes(
        kind: TextureKind,
        pixel_kind: TexturePixelKind,
        bytes: Vec<u8>,
        serialize_content: bool,
    ) -> Option<Self> {
        if bytes_in_first_mip(kind, pixel_kind) != bytes.len() as u32 {
            None
        } else {
            Some(Self {
                path: Default::default(),
                kind,
                data_hash: data_hash(&bytes),
                bytes: bytes.into(),
                pixel_kind,
                serialize_content,
                ..Default::default()
            })
        }
    }

    /// Sets new minification filter. It is used when texture becomes smaller.
    pub fn set_minification_filter(&mut self, filter: TextureMinificationFilter) {
        self.minification_filter = filter;
    }

    /// Returns current minification filter.
    pub fn minification_filter(&self) -> TextureMinificationFilter {
        self.minification_filter
    }

    /// Sets new magnification filter. It is used when texture is "stretching".
    pub fn set_magnification_filter(&mut self, filter: TextureMagnificationFilter) {
        self.magnification_filter = filter;
    }

    /// Returns current magnification filter.
    pub fn magnification_filter(&self) -> TextureMagnificationFilter {
        self.magnification_filter
    }

    /// Sets new S coordinate wrap mode.
    pub fn set_s_wrap_mode(&mut self, s_wrap_mode: TextureWrapMode) {
        self.s_wrap_mode = s_wrap_mode;
    }

    /// Returns current S coordinate wrap mode.
    pub fn s_wrap_mode(&self) -> TextureWrapMode {
        self.s_wrap_mode
    }

    /// Sets new T coordinate wrap mode.
    pub fn set_t_wrap_mode(&mut self, t_wrap_mode: TextureWrapMode) {
        self.t_wrap_mode = t_wrap_mode;
    }

    /// Returns current T coordinate wrap mode.
    pub fn t_wrap_mode(&self) -> TextureWrapMode {
        self.t_wrap_mode
    }

    /// Returns total mip count.
    pub fn mip_count(&self) -> u32 {
        self.mip_count
    }

    /// Returns texture kind.
    pub fn kind(&self) -> TextureKind {
        self.kind
    }

    /// Returns current data hash. Hash is guaranteed to be in actual state.
    pub fn data_hash(&self) -> u64 {
        self.data_hash
    }

    /// Returns current pixel kind.
    pub fn pixel_kind(&self) -> TexturePixelKind {
        self.pixel_kind
    }

    /// Returns current data as immutable slice.
    pub fn data(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns data of the first mip level.
    pub fn first_mip_level_data(&self) -> &[u8] {
        &self.bytes[0..bytes_in_first_mip(self.kind, self.pixel_kind) as usize]
    }

    /// Returns true if the texture is procedural, false - otherwise.
    ///
    /// # Notes
    ///
    /// Content of procedural textures is saved during serialization and they never resolved
    /// on deserialization. Resolving here means a process of getting correct texture instance
    /// by its path.
    pub fn is_procedural(&self) -> bool {
        self.serialize_content
    }

    /// Returns true if the texture is used as render target.
    pub fn is_render_target(&self) -> bool {
        self.is_render_target
    }

    /// Max samples for anisotropic filtering. Default value is 16.0 (max).
    /// However real value passed to GPU will be clamped to maximum supported
    /// by current GPU. To disable anisotropic filtering set this to 1.0.
    /// Typical values are 2.0, 4.0, 8.0, 16.0.
    pub fn set_anisotropy_level(&mut self, anisotropy: f32) {
        self.anisotropy = anisotropy.max(1.0);
    }

    /// Returns current anisotropy level.
    pub fn anisotropy_level(&self) -> f32 {
        self.anisotropy
    }

    /// Sets new path to source file.
    pub fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path = path.as_ref().to_owned();
    }

    /// Tries to save internal buffer into source file.
    pub fn save(&self) -> Result<(), TextureError> {
        let color_type = match self.pixel_kind {
            TexturePixelKind::R8 => ColorType::L8,
            TexturePixelKind::Luminance8 => ColorType::L8,
            TexturePixelKind::RGB8 => ColorType::Rgb8,
            TexturePixelKind::RGBA8 => ColorType::Rgba8,
            TexturePixelKind::RG8 => ColorType::La8,
            TexturePixelKind::LuminanceAlpha8 => ColorType::La8,
            TexturePixelKind::R16 => ColorType::L16,
            TexturePixelKind::Luminance16 => ColorType::L16,
            TexturePixelKind::RG16 => ColorType::La16,
            TexturePixelKind::LuminanceAlpha16 => ColorType::La16,
            TexturePixelKind::RGB16 => ColorType::Rgb16,
            TexturePixelKind::RGBA16 => ColorType::Rgba16,
            TexturePixelKind::RGB32F => ColorType::Rgb32F,
            TexturePixelKind::RGBA32F => ColorType::Rgba32F,
            TexturePixelKind::DXT1RGB
            | TexturePixelKind::DXT1RGBA
            | TexturePixelKind::DXT3RGBA
            | TexturePixelKind::DXT5RGBA
            | TexturePixelKind::R8RGTC
            | TexturePixelKind::RG8RGTC
            | TexturePixelKind::BGR8
            | TexturePixelKind::BGRA8 => return Err(TextureError::UnsupportedFormat),
        };
        if let TextureKind::Rectangle { width, height } = self.kind {
            Ok(image::save_buffer(
                &self.path,
                self.bytes.as_ref(),
                width,
                height,
                color_type,
            )?)
        } else {
            Err(TextureError::UnsupportedFormat)
        }
    }

    /// Returns a special reference holder that provides mutable access to content of the
    /// texture and automatically calculates hash of the data in its destructor.
    pub fn modify(&mut self) -> TextureDataRefMut<'_> {
        TextureDataRefMut { texture: self }
    }
}

/// A special reference holder that provides mutable access to content of the
/// texture and automatically calculates hash of the data in its destructor.
pub struct TextureDataRefMut<'a> {
    texture: &'a mut TextureData,
}

impl<'a> Drop for TextureDataRefMut<'a> {
    fn drop(&mut self) {
        self.texture.data_hash = data_hash(&self.texture.bytes);
    }
}

impl<'a> Deref for TextureDataRefMut<'a> {
    type Target = TextureData;

    fn deref(&self) -> &Self::Target {
        self.texture
    }
}

impl<'a> DerefMut for TextureDataRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.texture
    }
}

impl<'a> TextureDataRefMut<'a> {
    /// Returns mutable reference to the data of the texture.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.texture.bytes
    }
}

#[cfg(test)]
pub mod test {
    use crate::resource::texture::{Texture, TextureKind, TexturePixelKind};

    pub fn create_test_texture() -> Texture {
        Texture::from_bytes(
            TextureKind::Rectangle {
                width: 1,
                height: 1,
            },
            TexturePixelKind::RGBA8,
            vec![1, 1, 1, 1],
            false,
        )
        .unwrap()
    }
}
