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

use crate::core::visitor::{Visit, VisitError, VisitResult, Visitor};
use image::{ColorType, DynamicImage, GenericImageView, ImageError};
use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex, MutexGuard},
    task::{Context, Poll, Waker},
};

/// Actual texture data.
#[derive(Debug)]
pub struct TextureDetails {
    pub(in crate) path: PathBuf,
    pub(in crate) width: u32,
    pub(in crate) height: u32,
    pub(in crate) bytes: Vec<u8>,
    pub(in crate) kind: TextureKind,
    minification_filter: TextureMinificationFilter,
    magnification_filter: TextureMagnificationFilter,
    anisotropy: f32,
}

impl Default for TextureDetails {
    /// It is very important to mention that defaults may be different for texture when you
    /// importing them through resource manager, see
    /// [TextureImportOptions](../engine/resource_manager/struct.TextureImportOptions.html) for more info.
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            width: 0,
            height: 0,
            bytes: Vec::new(),
            kind: TextureKind::RGBA8,
            minification_filter: TextureMinificationFilter::LinearMipMapLinear,
            magnification_filter: TextureMagnificationFilter::Linear,
            anisotropy: 16.0,
        }
    }
}

/// Texture could be in three possible states:
/// 1. Pending - it is loading.
/// 2. LoadError - an error has occurred during the load.
/// 3. Ok - texture is fully loaded and ready to use.
///
/// Why it is so complex?
/// Short answer: asynchronous loading.
/// Long answer: when you loading a scene you expect it to be loaded as fast as
/// possible, use all available power of the CPU. To achieve that each texture
/// ideally should be loaded on separate core of the CPU, but since this is
/// asynchronous, we must have the ability to track the state of the texture.  
#[derive(Debug)]
pub enum TextureState {
    /// Texture is loading from external resource.
    Pending {
        /// A path to load texture from.
        path: PathBuf,
        /// List of wakers to wake future when texture is fully loaded.
        wakers: Vec<Waker>,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the texture.
        path: PathBuf,
        /// An error. This wrapped in Option only to be Default_ed.        
        error: Option<ImageError>,
    },
    /// Actual texture data when it is fully loaded or when texture was created procedurally.
    Ok(TextureDetails),
}

/// See module docs.
#[derive(Debug, Clone, Default)]
pub struct Texture {
    state: Option<Arc<Mutex<TextureState>>>,
}

impl From<Arc<Mutex<TextureState>>> for Texture {
    fn from(state: Arc<Mutex<TextureState>>) -> Self {
        Self { state: Some(state) }
    }
}

impl Into<Arc<Mutex<TextureState>>> for Texture {
    fn into(self) -> Arc<Mutex<TextureState>> {
        self.state.unwrap()
    }
}

impl Future for Texture {
    type Output = Self;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.as_ref().state.clone();
        match *state.unwrap().lock().unwrap() {
            TextureState::Pending { ref mut wakers, .. } => {
                // Collect wakers, so we'll be able to wake task when worker thread finish loading.
                let cx_waker = cx.waker();
                if let Some(pos) = wakers.iter().position(|waker| waker.will_wake(cx_waker)) {
                    wakers[pos] = cx_waker.clone();
                } else {
                    wakers.push(cx_waker.clone())
                }

                Poll::Pending
            }
            TextureState::LoadError { .. } | TextureState::Ok(_) => Poll::Ready(self.clone()),
        }
    }
}

impl Visit for TextureState {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        // This branch may fail only on load (except some extreme conditions like out of memory).
        if id.visit("Id", visitor).is_ok() {
            if visitor.is_reading() {
                *self = Self::from_id(id)?;
            }
            match self {
                // Unreachable because texture must be .await_ed before serialization.
                Self::Pending { .. } => unreachable!(),
                // This may look strange if we attempting to save an invalid texture, but this may be
                // actually useful - a texture may become loadable at the deserialization.
                Self::LoadError { path, .. } => path.visit("Path", visitor)?,
                Self::Ok(details) => details.visit("Details", visitor)?,
            }

            visitor.leave_region()
        } else {
            visitor.leave_region()?;

            // Keep compatibility with old versions.
            let mut details = TextureDetails::default();
            details.visit(name, visitor)?;

            *self = Self::Ok(details);
            Ok(())
        }
    }
}

impl Visit for Texture {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        // This branch may fail only on load (except some extreme conditions like out of memory).
        if self.state.visit("State", visitor).is_err() {
            visitor.leave_region()?;

            // Keep compatibility with old versions.

            // Create default state here, since it is only for compatibility we don't care about
            // redundant memory allocations.
            let mut state = Arc::new(Mutex::new(TextureState::Ok(Default::default())));

            state.visit(name, visitor)?;

            self.state = Some(state);

            Ok(())
        } else {
            visitor.leave_region()
        }
    }
}

impl Texture {
    /// Creates new texture with a given state.
    pub fn new(state: TextureState) -> Self {
        Self {
            state: Some(Arc::new(Mutex::new(state))),
        }
    }

    /// Converts self to internal value.
    pub fn into_inner(self) -> Arc<Mutex<TextureState>> {
        self.state.unwrap()
    }

    /// Locks internal mutex provides access to the state.
    pub fn state(&self) -> MutexGuard<'_, TextureState> {
        self.state.as_ref().unwrap().lock().unwrap()
    }

    /// Returns exact amount of users of the texture.
    pub fn use_count(&self) -> usize {
        Arc::strong_count(&self.state.as_ref().unwrap())
    }

    /// Returns a pointer as numeric value which can be used as a hash.
    pub fn key(&self) -> usize {
        (&**self.state.as_ref().unwrap() as *const _) as usize
    }

    /// Creates new render target for a scene. This method automatically configures GPU texture
    /// to correct settings, after render target was created, it must not be modified, otherwise
    /// result is undefined.
    pub fn new_render_target() -> Self {
        Self {
            state: Some(Arc::new(Mutex::new(TextureState::Ok(TextureDetails {
                path: Default::default(),
                width: 0,
                height: 0,
                bytes: Vec::new(),
                kind: TextureKind::RGBA8,
                minification_filter: TextureMinificationFilter::Nearest,
                magnification_filter: TextureMagnificationFilter::Nearest,
                anisotropy: 1.0,
            })))),
        }
    }
}

impl TextureState {
    fn id(&self) -> u32 {
        match self {
            Self::Pending { .. } => 0,
            Self::LoadError { .. } => 1,
            Self::Ok(_) => 2,
        }
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Pending {
                path: Default::default(),
                wakers: Default::default(),
            }),
            1 => Ok(Self::LoadError {
                path: Default::default(),
                error: None,
            }),
            2 => Ok(Self::Ok(Default::default())),
            _ => Err(format!("Invalid texture id {}", id)),
        }
    }

    /// Returns a path to the texture source.
    pub fn path(&self) -> &Path {
        match self {
            Self::Pending { path, .. } => path,
            Self::LoadError { path, .. } => path,
            Self::Ok(details) => &details.path,
        }
    }
}

impl Default for TextureState {
    fn default() -> Self {
        Self::Ok(Default::default())
    }
}

impl Visit for TextureDetails {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind = self.kind.id();
        kind.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = TextureKind::new(kind)?;
        }

        self.path.visit("Path", visitor)?;
        let _ = self
            .minification_filter
            .visit("MinificationFilter", visitor);
        let _ = self
            .magnification_filter
            .visit("MagnificationFilter", visitor);
        let _ = self.anisotropy.visit("Anisotropy", visitor);

        visitor.leave_region()
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

/// Texture kind defines pixel format of texture.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum TextureKind {
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

impl TextureKind {
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

impl TextureDetails {
    pub(in crate) fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, image::ImageError> {
        let dyn_img = image::open(path.as_ref())?;

        let width = dyn_img.width();
        let height = dyn_img.height();

        let kind = match dyn_img {
            DynamicImage::ImageLuma8(_) => TextureKind::R8,
            DynamicImage::ImageLumaA8(_) => TextureKind::RG8,
            DynamicImage::ImageRgb8(_) => TextureKind::RGB8,
            DynamicImage::ImageRgba8(_) => TextureKind::RGBA8,
            DynamicImage::ImageBgr8(_) => TextureKind::BGR8,
            DynamicImage::ImageBgra8(_) => TextureKind::BGRA8,
            DynamicImage::ImageLuma16(_) => TextureKind::R16,
            DynamicImage::ImageLumaA16(_) => TextureKind::RG16,
            DynamicImage::ImageRgb16(_) => TextureKind::RGB16,
            DynamicImage::ImageRgba16(_) => TextureKind::RGBA16,
        };

        Ok(Self {
            kind,
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
        kind: TextureKind,
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
                kind,
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
        let color_type = match self.kind {
            TextureKind::R8 => ColorType::L8,
            TextureKind::RGB8 => ColorType::Rgb8,
            TextureKind::RGBA8 => ColorType::Rgba8,
            TextureKind::RG8 => ColorType::La8,
            TextureKind::R16 => ColorType::L16,
            TextureKind::RG16 => ColorType::La16,
            TextureKind::BGR8 => ColorType::Bgr8,
            TextureKind::BGRA8 => ColorType::Bgra8,
            TextureKind::RGB16 => ColorType::Rgb16,
            TextureKind::RGBA16 => ColorType::Rgba16,
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
