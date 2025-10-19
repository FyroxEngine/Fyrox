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

//! User Interface loader.

use crate::{
    constructor::new_widget_constructor_container,
    core::{uuid::Uuid, TypeUuidProvider},
    UserInterface,
};
use fyrox_core::{
    io::FileError,
    platform::TargetPlatform,
    visitor::{Format, Visitor},
};
use fyrox_resource::{
    io::ResourceIo,
    loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    manager::ResourceManager,
    state::LoadError,
};
use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};

/// Default implementation for UI loading.
pub struct UserInterfaceLoader {
    pub resource_manager: ResourceManager,
}

impl ResourceLoader for UserInterfaceLoader {
    fn extensions(&self) -> &[&str] {
        &["ui"]
    }

    fn is_native_extension(&self, ext: &str) -> bool {
        fyrox_core::cmp_strings_case_insensitive(ext, "ui")
    }

    fn data_type_uuid(&self) -> Uuid {
        UserInterface::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        Box::pin(async move {
            let io = io.as_ref();
            let ui = UserInterface::load_from_file_ex(
                &path,
                Arc::new(new_widget_constructor_container()),
                resource_manager,
                io,
            )
            .await
            .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(ui))
        })
    }

    fn convert(
        &self,
        src_path: PathBuf,
        dest_path: PathBuf,
        _platform: TargetPlatform,
        io: Arc<dyn ResourceIo>,
    ) -> Pin<Box<dyn Future<Output = Result<(), FileError>>>> {
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
    }
}
