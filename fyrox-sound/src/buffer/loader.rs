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

//! Sound buffer loader.

use crate::buffer::{DataSource, SoundBuffer};
use fyrox_core::{reflect::prelude::*, uuid::Uuid, TypeUuidProvider};
use fyrox_resource::{
    io::ResourceIo,
    loader::{BoxedImportOptionsLoaderFuture, BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    options::{
        try_get_import_settings, try_get_import_settings_opaque, BaseImportOptions, ImportOptions,
    },
    state::LoadError,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

/// Defines sound buffer resource import options.
#[derive(Clone, Deserialize, Serialize, Default, Debug, Reflect)]
pub struct SoundBufferImportOptions {
    /// Whether the buffer is streaming or not.
    pub stream: bool,
}

impl ImportOptions for SoundBufferImportOptions {}

/// Default implementation for sound buffer loading.
pub struct SoundBufferLoader {
    /// Default import options for sound buffer resources.
    pub default_import_options: SoundBufferImportOptions,
}

impl ResourceLoader for SoundBufferLoader {
    fn extensions(&self) -> &[&str] {
        &["wav", "ogg"]
    }

    fn data_type_uuid(&self) -> Uuid {
        SoundBuffer::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let default_import_options = self.default_import_options.clone();

        Box::pin(async move {
            let io = io.as_ref();

            let import_options = try_get_import_settings(&path, io)
                .await
                .unwrap_or(default_import_options);

            let source = DataSource::from_file(&path, io)
                .await
                .map_err(LoadError::new)?;

            let result = if import_options.stream {
                SoundBuffer::raw_streaming(source)
            } else {
                SoundBuffer::raw_generic(source)
            };

            match result {
                Ok(buffer) => Ok(LoaderPayload::new(buffer)),
                Err(_) => Err(LoadError::new("Invalid data source.")),
            }
        })
    }

    fn try_load_import_settings(
        &self,
        resource_path: PathBuf,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedImportOptionsLoaderFuture {
        Box::pin(async move {
            try_get_import_settings_opaque::<SoundBufferImportOptions>(&resource_path, &*io).await
        })
    }

    fn default_import_options(&self) -> Option<Box<dyn BaseImportOptions>> {
        Some(Box::<SoundBufferImportOptions>::default())
    }
}
