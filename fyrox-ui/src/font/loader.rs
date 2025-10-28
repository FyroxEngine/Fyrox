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

//! Font loader.

use crate::{
    core::{reflect::prelude::*, uuid::Uuid, TypeUuidProvider},
    font::{Font, FontResource},
};
use fyrox_resource::{
    io::ResourceIo,
    loader::{BoxedImportOptionsLoaderFuture, BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    manager::ResourceManager,
    options::{
        try_get_import_settings, try_get_import_settings_opaque, BaseImportOptions, ImportOptions,
    },
    state::LoadError,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

fn default_page_size() -> usize {
    1024
}

/// Options to control how a font is imported, allowing data to be included beyond
/// what is stored in the font file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect, Eq)]
pub struct FontImportOptions {
    /// The size of each page of the atlas where glyphs are copied before they are rendered.
    /// Each page is a square `page_size` x `page_size` large, and no glyph may be so large
    /// that it cannot fit in that square or else it will fail to render.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    /// The bold version of this font.
    pub bold: Option<FontResource>,
    /// The italic version of this font.
    pub italic: Option<FontResource>,
    /// The bold italic version of this font.
    pub bold_italic: Option<FontResource>,
    /// Fallback fonts are used for rendering special characters that do not have glyphs in this
    /// font.
    pub fallbacks: Vec<Option<FontResource>>,
}

impl Default for FontImportOptions {
    fn default() -> Self {
        Self {
            page_size: default_page_size(),
            bold: None,
            italic: None,
            bold_italic: None,
            fallbacks: Vec::default(),
        }
    }
}

impl ImportOptions for FontImportOptions {}

/// Default implementation for font loading.
pub struct FontLoader {
    /// Resource manager to allow fallback font loading.
    pub resource_manager: ResourceManager,
    default_import_options: FontImportOptions,
}

impl FontLoader {
    /// Create an instance of the font loader using the given resource manager.
    pub fn new(resource_manager: ResourceManager) -> Self {
        Self {
            resource_manager,
            default_import_options: FontImportOptions::default(),
        }
    }
}

impl ResourceLoader for FontLoader {
    fn extensions(&self) -> &[&str] {
        &["ttf", "otf"]
    }

    fn data_type_uuid(&self) -> Uuid {
        Font::type_uuid()
    }

    fn default_import_options(&self) -> Option<Box<dyn BaseImportOptions>> {
        Some(Box::new(self.default_import_options.clone()))
    }

    fn try_load_import_settings(
        &self,
        resource_path: PathBuf,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedImportOptionsLoaderFuture {
        Box::pin(async move {
            try_get_import_settings_opaque::<FontImportOptions>(&resource_path, &*io).await
        })
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let default_import_options = self.default_import_options.clone();
        let resource_manager = self.resource_manager.clone();
        Box::pin(async move {
            let io = io.as_ref();

            let import_options = try_get_import_settings(&path, io)
                .await
                .unwrap_or(default_import_options);

            let font = Font::from_file(&path, import_options, io, &resource_manager)
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(font))
        })
    }
}
