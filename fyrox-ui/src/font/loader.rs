//! Font loader.

use crate::{
    core::{reflect::prelude::*, uuid::Uuid, TypeUuidProvider},
    font::Font,
};
use fyrox_resource::{
    io::ResourceIo,
    loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    options::{try_get_import_settings, ImportOptions},
    state::LoadError,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

fn default_page_size() -> usize {
    1024
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Reflect, Eq)]
pub struct FontImportOptions {
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

impl Default for FontImportOptions {
    fn default() -> Self {
        Self {
            page_size: default_page_size(),
        }
    }
}

impl ImportOptions for FontImportOptions {}

/// Default implementation for font loading.
#[derive(Default)]
pub struct FontLoader {
    default_import_options: FontImportOptions,
}

impl ResourceLoader for FontLoader {
    fn extensions(&self) -> &[&str] {
        &["ttf", "otf"]
    }

    fn data_type_uuid(&self) -> Uuid {
        Font::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let default_import_options = self.default_import_options.clone();
        Box::pin(async move {
            let io = io.as_ref();

            let import_options = try_get_import_settings(&path, io)
                .await
                .unwrap_or(default_import_options);

            let font = Font::from_file(&path, import_options.page_size, io)
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(font))
        })
    }
}
