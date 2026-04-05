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

use crate::{TextureMagnificationFilter, TextureMinificationFilter, TextureWrapMode};
use fyrox_core::sparse::AtomicIndex;
use fyrox_core::{
    io::FileError, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
};
use fyrox_resource::builtin::BuiltInResource;
use fyrox_resource::untyped::ResourceKind;
use fyrox_resource::{
    io::ResourceIo,
    loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    state::LoadError,
    Resource, ResourceData,
};
use std::sync::LazyLock;
use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
    sync::Arc,
};

/// A set of potential options that can be used to configure a GPU sampler.
#[derive(Visit, Reflect, TypeUuidProvider, Clone, Debug)]
#[type_uuid(id = "f04581da-9a28-4afa-961d-b9ed6f8c6c81")]
pub struct TextureSampler {
    /// Minification filter of the texture. See [`TextureMinificationFilter`] docs for more info.
    pub min_filter: TextureMinificationFilter,
    /// Magnification filter of the texture. See [`TextureMagnificationFilter`] docs for more info.
    pub mag_filter: TextureMagnificationFilter,
    /// S coordinate wrap mode. See [`TextureWrapMode`] docs for more info.
    pub s_wrap_mode: TextureWrapMode,
    /// T coordinate wrap mode. See [`TextureWrapMode`] docs for more info.
    pub t_wrap_mode: TextureWrapMode,
    /// R coordinate wrap mode. See [`TextureWrapMode`] docs for more info.
    pub r_wrap_mode: TextureWrapMode,
    /// Anisotropy level of the texture. Default is 1.0. Max number is usually depends on the
    /// GPU, but the cap is 16.0 on pretty much any platform. This number should be a power of two.
    pub anisotropy: f32,
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

    pub modification_count: u64,

    #[doc(hidden)]
    #[visit(skip)]
    #[reflect(hidden)]
    pub cache_index: Arc<AtomicIndex>,
}

impl Default for TextureSampler {
    fn default() -> Self {
        Self {
            min_filter: Default::default(),
            mag_filter: Default::default(),
            s_wrap_mode: Default::default(),
            t_wrap_mode: Default::default(),
            r_wrap_mode: Default::default(),
            anisotropy: 1.0,
            min_lod: -1000.0,
            max_lod: 1000.0,
            lod_bias: 0.0,
            modification_count: 0,
            cache_index: Arc::new(Default::default()),
        }
    }
}

/// A default sampler with linear filtration (mip-mapped), "repeat" mode wrapping on each axis,
/// anisotropy of 1.0.
pub static STANDARD: LazyLock<BuiltInResource<TextureSampler>> = LazyLock::new(|| {
    BuiltInResource::new_no_source(
        "Default Sampler",
        TextureSamplerResource::new_ok(
            uuid!("61ddcdf5-acd9-418b-a0d6-0d09bcad8242"),
            ResourceKind::External,
            TextureSampler::default(),
        ),
    )
});

impl ResourceData for TextureSampler {
    fn type_uuid(&self) -> Uuid {
        <TextureSampler as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut visitor = Visitor::new();
        self.visit("Sampler", &mut visitor)?;
        visitor.save_ascii_to_file(path)?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }

    fn try_clone_box(&self) -> Option<Box<dyn ResourceData>> {
        Some(Box::new(self.clone()))
    }
}

/// An error that may occur during curve resource loading.
#[derive(Debug)]
pub enum TextureSamplerResourceError {
    /// An i/o error has occurred.
    Io(FileError),

    /// An error that may occur due to version incompatibilities.
    Visit(VisitError),
}

impl Display for TextureSamplerResourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TextureSamplerResourceError::Io(v) => {
                write!(f, "A file load error has occurred {v:?}")
            }
            TextureSamplerResourceError::Visit(v) => {
                write!(
                    f,
                    "An error that may occur due to version incompatibilities. {v:?}"
                )
            }
        }
    }
}

impl From<FileError> for TextureSamplerResourceError {
    fn from(e: FileError) -> Self {
        Self::Io(e)
    }
}

impl From<VisitError> for TextureSamplerResourceError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

impl TextureSampler {
    /// Load a curve resource from the specific file path.
    pub async fn from_file(
        path: &Path,
        io: &dyn ResourceIo,
    ) -> Result<Self, TextureSamplerResourceError> {
        let bytes = io.load_file(path).await?;
        let mut visitor = Visitor::load_from_memory(&bytes)?;
        let mut sampler = TextureSampler::default();
        sampler.visit("Sampler", &mut visitor)?;
        Ok(sampler)
    }

    /// Sets new minification filter. It is used when texture becomes smaller.
    #[inline]
    pub fn set_minification_filter(&mut self, filter: TextureMinificationFilter) {
        self.min_filter = filter;
        self.modification_count += 1;
    }

    /// Returns current minification filter.
    #[inline]
    pub fn minification_filter(&self) -> TextureMinificationFilter {
        self.min_filter
    }

    /// Sets new magnification filter. It is used when texture is "stretching".
    #[inline]
    pub fn set_magnification_filter(&mut self, filter: TextureMagnificationFilter) {
        self.mag_filter = filter;
        self.modification_count += 1;
    }

    /// Returns current magnification filter.
    #[inline]
    pub fn magnification_filter(&self) -> TextureMagnificationFilter {
        self.mag_filter
    }

    /// Sets new S coordinate wrap mode.
    #[inline]
    pub fn set_s_wrap_mode(&mut self, s_wrap_mode: TextureWrapMode) {
        self.s_wrap_mode = s_wrap_mode;
        self.modification_count += 1;
    }

    /// Returns current S coordinate wrap mode.
    #[inline]
    pub fn s_wrap_mode(&self) -> TextureWrapMode {
        self.s_wrap_mode
    }

    /// Sets new T coordinate wrap mode.
    #[inline]
    pub fn set_t_wrap_mode(&mut self, t_wrap_mode: TextureWrapMode) {
        self.t_wrap_mode = t_wrap_mode;
        self.modification_count += 1;
    }

    /// Returns current T coordinate wrap mode.
    #[inline]
    pub fn t_wrap_mode(&self) -> TextureWrapMode {
        self.t_wrap_mode
    }

    /// Sets new R coordinate wrap mode.
    #[inline]
    pub fn set_r_wrap_mode(&mut self, r_wrap_mode: TextureWrapMode) {
        self.r_wrap_mode = r_wrap_mode;
        self.modification_count += 1;
    }

    pub fn modifications_count(&self) -> u64 {
        self.modification_count
    }

    /// Returns current T coordinate wrap mode.
    #[inline]
    pub fn r_wrap_mode(&self) -> TextureWrapMode {
        self.r_wrap_mode
    }

    /// Returns the minimum level-of-detail parameter. See [`Self::set_min_lod`] for more info.
    #[inline]
    pub fn min_lod(&self) -> f32 {
        self.min_lod
    }

    /// Sets the minimum level-of-detail parameter. This floating-point value limits the selection
    /// of highest resolution mipmap (lowest mipmap level). The initial value is -1000.0.
    #[inline]
    pub fn set_min_lod(&mut self, min_lod: f32) {
        self.modification_count += 1;
        self.min_lod = min_lod;
    }

    /// Returns the maximum level-of-detail parameter. See [`Self::set_max_lod`] for more info.
    #[inline]
    pub fn max_lod(&self) -> f32 {
        self.max_lod
    }

    /// Sets the maximum level-of-detail parameter. This floating-point value limits the selection
    /// of the lowest resolution mipmap (highest mipmap level). The initial value is 1000.
    #[inline]
    pub fn set_max_lod(&mut self, max_lod: f32) {
        self.modification_count += 1;
        self.max_lod = max_lod;
    }

    /// Returns a fixed bias value that is to be added to the level-of-detail parameter for the
    /// texture before texture sampling. See [`Self::set_lod_bias`] for more info.
    #[inline]
    pub fn lod_bias(&self) -> f32 {
        self.lod_bias
    }

    /// Specifies a fixed bias value that is to be added to the level-of-detail parameter for the
    /// texture before texture sampling. The specified value is added to the shader-supplied bias
    /// value (if any) and subsequently clamped into the implementation-defined range
    /// `−bias_max..bias_max`, where `bias_max` is the value that can be fetched from the current
    /// graphics server. The initial value is 0.0.
    #[inline]
    pub fn set_lod_bias(&mut self, lod_bias: f32) {
        self.modification_count += 1;
        self.lod_bias = lod_bias;
    }

    /// Max samples for anisotropic filtering. Default value is 16.0 (max).
    /// However real value passed to GPU will be clamped to maximum supported
    /// by current GPU. To disable anisotropic filtering set this to 1.0.
    /// Typical values are 2.0, 4.0, 8.0, 16.0.
    #[inline]
    pub fn set_anisotropy_level(&mut self, anisotropy: f32) {
        self.modification_count += 1;
        self.anisotropy = anisotropy.max(1.0);
    }

    /// Returns current anisotropy level.
    #[inline]
    pub fn anisotropy_level(&self) -> f32 {
        self.anisotropy
    }
}

pub type TextureSamplerResource = Resource<TextureSampler>;

pub struct TextureSamplerLoader;

impl ResourceLoader for TextureSamplerLoader {
    fn extensions(&self) -> &[&str] {
        &["sampler"]
    }

    fn data_type_uuid(&self) -> Uuid {
        <TextureSampler as TypeUuidProvider>::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        Box::pin(async move {
            let texture_sampler = TextureSampler::from_file(&path, io.as_ref())
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(texture_sampler))
        })
    }
}
