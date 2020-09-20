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
//! Texture can be used as render target to render scene in it. To do this you should make
//! default instance of a texture and pass it to scene's render target property. Renderer
//! will automatically provide you info about metrics of texture, but it won't give you
//! access to pixels of render target.

use crate::core::visitor::{Visit, VisitResult, Visitor};
use image::{ColorType, GenericImageView, ImageError};
use std::path::{Path, PathBuf};

/// See module docs.
#[derive(Debug)]
pub struct Texture {
    pub(in crate) path: PathBuf,
    pub(in crate) width: u32,
    pub(in crate) height: u32,
    pub(in crate) bytes: Vec<u8>,
    pub(in crate) kind: TextureKind,
    pub(in crate) loaded: bool,
}

impl Default for Texture {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            width: 0,
            height: 0,
            bytes: Vec::new(),
            kind: TextureKind::RGBA8,
            loaded: true,
        }
    }
}

impl Visit for Texture {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind = self.kind.id();
        kind.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = TextureKind::new(kind)?;
        }

        self.path.visit("Path", visitor)?;

        visitor.leave_region()
    }
}

/// Texture kind defines pixel format of texture.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TextureKind {
    /// Only red component as 1 byte.
    R8,
    /// Red, green, and blue components, each by 1 byte.
    RGB8,
    /// Red, green, blue, and alpha components, each by 1 byte.
    RGBA8,
}

impl TextureKind {
    fn new(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::R8),
            1 => Ok(Self::RGB8),
            2 => Ok(Self::RGBA8),
            _ => Err(format!("Invalid texture kind {}!", id)),
        }
    }

    fn id(self) -> u32 {
        match self {
            Self::R8 => 0,
            Self::RGB8 => 1,
            Self::RGBA8 => 2,
        }
    }

    fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8 => 1,
            Self::RGB8 => 3,
            Self::RGBA8 => 4,
        }
    }
}

impl Texture {
    pub(in crate) fn load_from_file<P: AsRef<Path>>(
        path: P,
        kind: TextureKind,
    ) -> Result<Self, image::ImageError> {
        let dyn_img = image::open(path.as_ref())?;

        let width = dyn_img.width();
        let height = dyn_img.height();

        let bytes = match kind {
            TextureKind::R8 => dyn_img.to_luma().into_raw(),
            TextureKind::RGB8 => dyn_img.to_rgb().into_raw(),
            TextureKind::RGBA8 => dyn_img.to_rgba().into_raw(),
        };

        Ok(Self {
            kind,
            width,
            height,
            bytes,
            path: path.as_ref().to_path_buf(),
            loaded: true,
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
                loaded: true,
            })
        }
    }

    /// Returns true if texture is loaded. This is hacky method to support poorman's async
    /// texture loading. This will be changed in future. For now this is a TODO.
    pub fn is_loaded(&self) -> bool {
        self.loaded
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
