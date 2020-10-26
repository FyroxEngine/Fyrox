//! Texture is an image that used to fill faces to add details to them.
//!
//! In most cases textures are just 2D images, however there are some exclusions to that -
//! for example cube maps, that may be used for environment mapping. For now only 2D textures
//! are supported.
//!
//! # Supported formats
//!
//! To load images and decode them, rg3d uses image create which supports following image
//! formats: png, tga, bmp, dds, jpg, gif, tiff, dxt.
//!
//! # Render target
//!
//! Texture can be used as render target to render scene in it. To do this you should use
//! new_render_target method and pass its result to scene's render target property. Renderer
//! will automatically provide you info about metrics of texture, but it won't give you
//! access to pixels of render target.

use crate::{
    core::visitor::{Visit, VisitError, VisitResult, Visitor},
    resource::{Resource, ResourceData, ResourceState},
};
use image::{ColorType, DynamicImage, GenericImageView, ImageError};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// Actual texture data.
#[derive(Debug)]
pub struct TextureData {
    pub(in crate) path: PathBuf,
    pub(in crate) width: u32,
    pub(in crate) height: u32,
    pub(in crate) bytes: Vec<u8>,
    pub(in crate) pixel_kind: TexturePixelKind,
    minification_filter: TextureMinificationFilter,
    magnification_filter: TextureMagnificationFilter,
    s_wrap_mode: TextureWrapMode,
    t_wrap_mode: TextureWrapMode,
    anisotropy: f32,
}

impl ResourceData for TextureData {
    fn path(&self) -> Cow<Path> {
        Cow::Borrowed(&self.path)
    }
}

impl Visit for TextureData {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind = self.pixel_kind.id();
        kind.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.pixel_kind = TexturePixelKind::new(kind)?;
        }

        self.path.visit("Path", visitor)?;

        // Ignore result for backward compatibility.
        let _ = self
            .minification_filter
            .visit("MinificationFilter", visitor);
        let _ = self
            .magnification_filter
            .visit("MagnificationFilter", visitor);
        let _ = self.anisotropy.visit("Anisotropy", visitor);
        let _ = self.s_wrap_mode.visit("SWrapMode", visitor);
        let _ = self.t_wrap_mode.visit("TWrapMode", visitor);

        visitor.leave_region()
    }
}

impl Default for TextureData {
    /// It is very important to mention that defaults may be different for texture when you
    /// importing them through resource manager, see
    /// [TextureImportOptions](../engine/resource_manager/struct.TextureImportOptions.html) for more info.
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            width: 0,
            height: 0,
            bytes: Vec::new(),
            pixel_kind: TexturePixelKind::RGBA8,
            minification_filter: TextureMinificationFilter::LinearMipMapLinear,
            magnification_filter: TextureMagnificationFilter::Linear,
            s_wrap_mode: TextureWrapMode::Repeat,
            t_wrap_mode: TextureWrapMode::Repeat,
            anisotropy: 16.0,
        }
    }
}

/// See module docs.
pub type Texture = Resource<TextureData, ImageError>;

/// Texture state alias.
pub type TextureState = ResourceState<TextureData, ImageError>;

impl Texture {
    /// Creates new render target for a scene. This method automatically configures GPU texture
    /// to correct settings, after render target was created, it must not be modified, otherwise
    /// result is undefined.
    pub fn new_render_target() -> Self {
        Self::new(TextureState::Ok(TextureData {
            path: Default::default(),
            // Render target will automatically set width and height before rendering.
            width: 0,
            height: 0,
            bytes: Vec::new(),
            pixel_kind: TexturePixelKind::RGBA8,
            minification_filter: TextureMinificationFilter::Nearest,
            magnification_filter: TextureMagnificationFilter::Nearest,
            s_wrap_mode: TextureWrapMode::ClampToEdge,
            t_wrap_mode: TextureWrapMode::ClampToEdge,
            anisotropy: 1.0,
        }))
    }
}

/// The texture magnification function is used when the pixel being textured maps to an area
/// less than or equal to one texture element.
#[derive(Copy, Clone, Debug, Hash, PartialOrd, PartialEq)]
#[repr(u32)]
pub enum TextureMagnificationFilter {
    /// Returns the value of the texture element that is nearest to the center of the pixel
    /// being textured.
    Nearest = 0,

    /// Returns the weighted average of the four texture elements that are closest to the
    /// center of the pixel being textured.
    Linear = 1,
}

impl Visit for TextureMagnificationFilter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = *self as u32;
        id.visit("Id", visitor)?;

        if visitor.is_reading() {
            *self = match id {
                0 => TextureMagnificationFilter::Nearest,
                1 => TextureMagnificationFilter::Linear,
                _ => {
                    return VisitResult::Err(VisitError::User(format!(
                        "Invalid magnification filter {}!",
                        id
                    )))
                }
            }
        }

        visitor.leave_region()
    }
}

/// The texture minifying function is used whenever the pixel being textured maps to an area
/// greater than one texture element.
#[derive(Copy, Clone, Debug, Hash, PartialOrd, PartialEq)]
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

impl Visit for TextureMinificationFilter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = *self as u32;
        id.visit("Id", visitor)?;

        if visitor.is_reading() {
            *self = match id {
                0 => TextureMinificationFilter::Nearest,
                1 => TextureMinificationFilter::NearestMipMapNearest,
                2 => TextureMinificationFilter::NearestMipMapLinear,
                3 => TextureMinificationFilter::Linear,
                4 => TextureMinificationFilter::LinearMipMapNearest,
                5 => TextureMinificationFilter::LinearMipMapLinear,
                _ => {
                    return VisitResult::Err(VisitError::User(format!(
                        "Invalid minification filter {}!",
                        id
                    )))
                }
            }
        }

        visitor.leave_region()
    }
}

/// Defines a law of texture coordinate modification.
#[derive(Copy, Clone, Debug, Hash, PartialOrd, PartialEq)]
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

impl Visit for TextureWrapMode {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = *self as u32;
        id.visit("Id", visitor)?;

        if visitor.is_reading() {
            *self = match id {
                0 => TextureWrapMode::Repeat,
                1 => TextureWrapMode::ClampToEdge,
                2 => TextureWrapMode::ClampToBorder,
                3 => TextureWrapMode::MirroredRepeat,
                4 => TextureWrapMode::MirrorClampToEdge,
                _ => {
                    return VisitResult::Err(VisitError::User(format!("Invalid wrap mode {}!", id)))
                }
            }
        }

        visitor.leave_region()
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
            _ => Err(format!("Invalid texture kind {}!", id)),
        }
    }

    fn id(self) -> u32 {
        self as u32
    }

    fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8 => 1,
            Self::R16 | Self::RG8 => 2,
            Self::RGB8 | Self::BGR8 => 3,
            Self::RGBA8 | Self::BGRA8 | Self::RG16 => 4,
            Self::RGB16 => 6,
            Self::RGBA16 => 8,
        }
    }
}

impl TextureData {
    pub(in crate) fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, image::ImageError> {
        let dyn_img = image::open(path.as_ref())?;

        let width = dyn_img.width();
        let height = dyn_img.height();

        let kind = match dyn_img {
            DynamicImage::ImageLuma8(_) => TexturePixelKind::R8,
            DynamicImage::ImageLumaA8(_) => TexturePixelKind::RG8,
            DynamicImage::ImageRgb8(_) => TexturePixelKind::RGB8,
            DynamicImage::ImageRgba8(_) => TexturePixelKind::RGBA8,
            DynamicImage::ImageBgr8(_) => TexturePixelKind::BGR8,
            DynamicImage::ImageBgra8(_) => TexturePixelKind::BGRA8,
            DynamicImage::ImageLuma16(_) => TexturePixelKind::R16,
            DynamicImage::ImageLumaA16(_) => TexturePixelKind::RG16,
            DynamicImage::ImageRgb16(_) => TexturePixelKind::RGB16,
            DynamicImage::ImageRgba16(_) => TexturePixelKind::RGBA16,
        };

        Ok(Self {
            pixel_kind: kind,
            width,
            height,
            bytes: dyn_img.to_bytes(),
            path: path.as_ref().to_path_buf(),
            ..Default::default()
        })
    }

    /// Creates new texture instance from given parameters.
    pub fn from_bytes(
        width: u32,
        height: u32,
        kind: TexturePixelKind,
        bytes: Vec<u8>,
    ) -> Result<Self, ()> {
        let bpp = kind.bytes_per_pixel();
        let required_bytes = width * height * bpp;
        if required_bytes != bytes.len() as u32 {
            Err(())
        } else {
            Ok(Self {
                path: Default::default(),
                width,
                height,
                bytes,
                pixel_kind: kind,
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
    pub fn set_path<P: AsRef<Path>>(&mut self, path: &P) {
        self.path = path.as_ref().to_owned();
    }

    /// Tries to save internal buffer into source file.
    pub fn save(&self) -> Result<(), ImageError> {
        let color_type = match self.pixel_kind {
            TexturePixelKind::R8 => ColorType::L8,
            TexturePixelKind::RGB8 => ColorType::Rgb8,
            TexturePixelKind::RGBA8 => ColorType::Rgba8,
            TexturePixelKind::RG8 => ColorType::La8,
            TexturePixelKind::R16 => ColorType::L16,
            TexturePixelKind::RG16 => ColorType::La16,
            TexturePixelKind::BGR8 => ColorType::Bgr8,
            TexturePixelKind::BGRA8 => ColorType::Bgra8,
            TexturePixelKind::RGB16 => ColorType::Rgb16,
            TexturePixelKind::RGBA16 => ColorType::Rgba16,
        };
        image::save_buffer(
            &self.path,
            self.bytes.as_ref(),
            self.width,
            self.height,
            color_type,
        )
    }
}
