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

//! Model loader.

use crate::{
    asset::{
        io::ResourceIo,
        loader::{
            BoxedImportOptionsLoaderFuture, BoxedLoaderFuture, LoaderPayload, ResourceLoader,
        },
        manager::ResourceManager,
        options::{try_get_import_settings, try_get_import_settings_opaque, BaseImportOptions},
    },
    core::{
        io::FileError,
        platform::TargetPlatform,
        uuid::Uuid,
        visitor::{Format, Visitor},
        TypeUuidProvider,
    },
    engine::SerializationContext,
    resource::model::{Model, ModelImportOptions},
};
use fyrox_resource::state::LoadError;
use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};

/// Default implementation for model loading.
pub struct ModelLoader {
    /// Resource manager to allow complex model loading.
    pub resource_manager: ResourceManager,
    /// Node constructors contains a set of constructors that allows to build a node using its
    /// type UUID.
    pub serialization_context: Arc<SerializationContext>,
    /// Default import options for model resources.
    pub default_import_options: ModelImportOptions,
}

impl ResourceLoader for ModelLoader {
    fn extensions(&self) -> &[&str] {
        &["rgs", "fbx"]
    }

    fn is_native_extension(&self, ext: &str) -> bool {
        fyrox_core::cmp_strings_case_insensitive("rgs", ext)
    }

    fn data_type_uuid(&self) -> Uuid {
        Model::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        let node_constructors = self.serialization_context.clone();
        let default_import_options = self.default_import_options.clone();

        Box::pin(async move {
            let io = io.as_ref();

            let import_options = try_get_import_settings(&path, io)
                .await
                .unwrap_or(default_import_options);

            let model = Model::load(
                path,
                io,
                node_constructors,
                resource_manager,
                import_options,
            )
            .await
            .map_err(LoadError::new)?;

            Ok(LoaderPayload::new(model))
        })
    }

    fn convert(
        &self,
        src_path: PathBuf,
        dest_path: PathBuf,
        _platform: TargetPlatform,
        io: Arc<dyn ResourceIo>,
    ) -> Pin<Box<dyn Future<Output = Result<(), FileError>>>> {
        if src_path.extension().is_some_and(|ext| {
            fyrox_core::cmp_strings_case_insensitive(ext.to_string_lossy(), "rgs")
        }) {
            // Convert scenes to the binary format where possible.
            Box::pin(async move {
                let data = io.load_file(&src_path).await?;
                match Visitor::detect_format_from_slice(&data) {
                    Format::Unknown => Err(FileError::Custom("Unknown format!".to_string())),
                    Format::Binary => {
                        // Copy the binary format as-is.
                        Ok(io.copy_file(&src_path, &dest_path).await?)
                    }
                    Format::Ascii => {
                        // Resave the ascii format as binary.
                        let visitor = Visitor::load_from_memory(&data).map_err(|err| {
                            FileError::Custom(format!(
                                "Unable to load {}. Reason: {err}",
                                src_path.display()
                            ))
                        })?;
                        visitor.save_binary_to_file(dest_path).map_err(|err| {
                            FileError::Custom(format!(
                                "Unable to save {}. Reason: {err}",
                                src_path.display()
                            ))
                        })?;
                        Ok(())
                    }
                }
            })
        } else {
            // FBX and other will be copied as is.
            Box::pin(async move { io.copy_file(&src_path, &dest_path).await })
        }
    }

    fn try_load_import_settings(
        &self,
        resource_path: PathBuf,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedImportOptionsLoaderFuture {
        Box::pin(async move {
            try_get_import_settings_opaque::<ModelImportOptions>(&resource_path, &*io).await
        })
    }

    fn default_import_options(&self) -> Option<Box<dyn BaseImportOptions>> {
        Some(Box::<ModelImportOptions>::default())
    }
}
