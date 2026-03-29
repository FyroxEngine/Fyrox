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
use fyrox_core::{
    io::FileError, reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*,
};
use fyrox_resource::{
    io::ResourceIo,
    loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    state::LoadError,
    Resource, ResourceData,
};
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
        }
    }
}

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
