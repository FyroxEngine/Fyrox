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

//! Texture loader.

use crate::{Texture, TextureImportOptions};
use fyrox_core::{uuid::Uuid, TypeUuidProvider};
use fyrox_resource::{
    io::ResourceIo, loader::BoxedImportOptionsLoaderFuture, loader::BoxedLoaderFuture,
    loader::LoaderPayload, loader::ResourceLoader, options::try_get_import_settings,
    options::try_get_import_settings_opaque, options::BaseImportOptions, state::LoadError,
};
use std::{path::PathBuf, sync::Arc};

/// Default implementation for texture loading.
pub struct TextureLoader {
    /// Default import options for textures.
    pub default_import_options: TextureImportOptions,
}

impl ResourceLoader for TextureLoader {
    fn extensions(&self) -> &[&str] {
        &[
            "jpg", "jpeg", "tga", "gif", "bmp", "png", "tiff", "tif", "dds",
        ]
    }

    fn data_type_uuid(&self) -> Uuid {
        Texture::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let default_import_options = self.default_import_options.clone();
        Box::pin(async move {
            let io = io.as_ref();

            let import_options = try_get_import_settings(&path, io)
                .await
                .unwrap_or(default_import_options);

            let raw_texture = Texture::load_from_file(&path, io, import_options)
                .await
                .map_err(LoadError::new)?;

            Ok(LoaderPayload::new(raw_texture))
        })
    }

    fn try_load_import_settings(
        &self,
        resource_path: PathBuf,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedImportOptionsLoaderFuture {
        Box::pin(async move {
            try_get_import_settings_opaque::<TextureImportOptions>(&resource_path, &*io).await
        })
    }

    fn default_import_options(&self) -> Option<Box<dyn BaseImportOptions>> {
        Some(Box::<TextureImportOptions>::default())
    }
}
