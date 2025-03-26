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

//! Contains all types related to shared style resource.

use crate::style::{IntoPrimitive, Style, StyleProperty, StyledProperty};
use fyrox_core::{
    io::FileError,
    log::Log,
    type_traits::prelude::*,
    visitor::{prelude::*, VisitError, Visitor},
    ImmutableString, Uuid,
};
use fyrox_resource::{
    io::ResourceIo,
    loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    manager::ResourceManager,
    state::LoadError,
    Resource, ResourceData,
};
use std::{
    error::Error,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
    sync::Arc,
};

/// An error that may occur during tile set resource loading.
#[derive(Debug)]
pub enum StyleResourceError {
    /// An i/o error has occurred.
    Io(FileError),

    /// An error that may occur due to version incompatibilities.
    Visit(VisitError),
}

impl Display for StyleResourceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(v) => {
                write!(f, "A file load error has occurred {v:?}")
            }
            Self::Visit(v) => {
                write!(
                    f,
                    "An error that may occur due to version incompatibilities. {v:?}"
                )
            }
        }
    }
}

impl From<FileError> for StyleResourceError {
    fn from(e: FileError) -> Self {
        Self::Io(e)
    }
}

impl From<VisitError> for StyleResourceError {
    fn from(e: VisitError) -> Self {
        Self::Visit(e)
    }
}

impl ResourceData for Style {
    fn type_uuid(&self) -> Uuid {
        <Self as TypeUuidProvider>::type_uuid()
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut visitor = Visitor::new();
        self.visit("Style", &mut visitor)?;
        visitor.save_binary(path)?;
        Ok(())
    }

    fn can_be_saved(&self) -> bool {
        true
    }
}

/// A loader for style resource.
pub struct StyleLoader {
    /// Resource manager handle.
    pub resource_manager: ResourceManager,
}

impl ResourceLoader for StyleLoader {
    fn extensions(&self) -> &[&str] {
        &["style"]
    }

    fn data_type_uuid(&self) -> Uuid {
        <Style as TypeUuidProvider>::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        Box::pin(async move {
            let tile_set = Style::from_file(&path, io.as_ref(), resource_manager)
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(tile_set))
        })
    }
}

/// Style resource.
pub type StyleResource = Resource<Style>;

/// Extension methods for [`StyleResource`].
pub trait StyleResourceExt {
    /// Same as [`Style::set`].
    fn set(&self, name: impl Into<ImmutableString>, property: impl Into<StyleProperty>);

    /// Same as [`Style::get`].
    fn get<P>(&self, name: impl Into<ImmutableString>) -> Option<P>
    where
        StyleProperty: IntoPrimitive<P>;

    /// Same as [`Style::get_or`].
    fn get_or<P>(&self, name: impl Into<ImmutableString>, default: P) -> P
    where
        StyleProperty: IntoPrimitive<P>;

    /// Same as [`Style::get_or_default`].
    fn get_or_default<P>(&self, name: impl Into<ImmutableString>) -> P
    where
        P: Default,
        StyleProperty: IntoPrimitive<P>;

    /// Same as [`Style::property`].
    fn property<P>(&self, name: impl Into<ImmutableString>) -> StyledProperty<P>
    where
        P: Default,
        StyleProperty: IntoPrimitive<P>;
}

impl StyleResourceExt for StyleResource {
    fn set(&self, name: impl Into<ImmutableString>, property: impl Into<StyleProperty>) {
        let mut state = self.state();
        if let Some(data) = state.data() {
            data.set(name, property);
        } else {
            Log::err("Unable to set style property, because the resource is invalid!")
        }
    }

    fn get<P>(&self, name: impl Into<ImmutableString>) -> Option<P>
    where
        StyleProperty: IntoPrimitive<P>,
    {
        let state = self.state();
        if let Some(data) = state.data_ref() {
            data.get(name)
        } else {
            Log::err("Unable to get style property, because the resource is invalid!");
            None
        }
    }

    fn get_or<P>(&self, name: impl Into<ImmutableString>, default: P) -> P
    where
        StyleProperty: IntoPrimitive<P>,
    {
        let state = self.state();
        if let Some(data) = state.data_ref() {
            data.get_or(name, default)
        } else {
            Log::err("Unable to get style property, because the resource is invalid!");
            default
        }
    }

    fn get_or_default<P>(&self, name: impl Into<ImmutableString>) -> P
    where
        P: Default,
        StyleProperty: IntoPrimitive<P>,
    {
        let state = self.state();
        if let Some(data) = state.data_ref() {
            data.get_or_default(name)
        } else {
            Log::err("Unable to get style property, because the resource is invalid!");
            P::default()
        }
    }

    fn property<P>(&self, name: impl Into<ImmutableString>) -> StyledProperty<P>
    where
        P: Default,
        StyleProperty: IntoPrimitive<P>,
    {
        let state = self.state();
        if let Some(data) = state.data_ref() {
            data.property(name)
        } else {
            Log::err("Unable to get style property, because the resource is invalid!");
            Default::default()
        }
    }
}
