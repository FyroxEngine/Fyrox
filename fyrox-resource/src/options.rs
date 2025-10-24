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

//! Resource import options common traits.

use crate::{
    core::{append_extension, log::Log},
    io::ResourceIo,
};
use fyrox_core::io::FileError;
use fyrox_core::reflect::Reflect;
use ron::ser::PrettyConfig;
use serde::{de::DeserializeOwned, Serialize};
use std::io::{ErrorKind, Write};
use std::{fs::File, path::Path};

/// Extension of import options file.
pub const OPTIONS_EXTENSION: &str = "options";

/// Base type-agnostic trait for resource import options. This trait has automatic implementation
/// for everything that implements [`ImportOptions`] trait.
pub trait BaseImportOptions: Reflect {
    /// Saves the options to a file at the given path.
    fn save(&self, path: &Path) -> bool;
}

/// A trait for resource import options. It provides generic functionality shared over all types of import options.
pub trait ImportOptions:
    BaseImportOptions + Serialize + DeserializeOwned + Default + Clone
{
    /// Saves import options into a specified file.
    fn save_internal(&self, path: &Path) -> bool {
        fn write<T: Serialize>(this: &T, path: &Path) -> std::io::Result<()> {
            let mut file = File::create(path)?;
            let string = ron::ser::to_string_pretty(this, PrettyConfig::default())
                .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))?;
            file.write_all(string.as_bytes())?;
            Ok(())
        }
        let result = write(self, path);
        let is_ok = result.is_ok();
        Log::verify(result);
        is_ok
    }
}

impl<T> BaseImportOptions for T
where
    T: ImportOptions,
{
    fn save(&self, path: &Path) -> bool {
        self.save_internal(path)
    }
}

/// Tries to load import settings for a resource. It is not part of ImportOptions trait because
/// `async fn` is not yet supported for traits.
pub async fn try_get_import_settings<T>(resource_path: &Path, io: &dyn ResourceIo) -> Option<T>
where
    T: ImportOptions,
{
    let settings_path = append_extension(resource_path, OPTIONS_EXTENSION);

    match io.load_file(settings_path.as_ref()).await {
        Ok(bytes) => match ron::de::from_bytes::<T>(&bytes) {
            Ok(options) => Some(options),
            Err(e) => {
                Log::warn(format!(
                    "Malformed options file {:?}, fallback to defaults! Reason: {}",
                    settings_path, e
                ));

                None
            }
        },
        Err(e) => {
            // Missing options file is a normal situation, the engine will use default import options
            // instead. Any other error indicates a real issue that needs to be highlighted to the
            // user.
            if let FileError::Io(ref err) = e {
                if err.kind() == ErrorKind::NotFound {
                    return None;
                }
            }

            Log::warn(format!(
                "Unable to load options file {:?}, fallback to defaults! Reason: {}",
                settings_path, e
            ));

            None
        }
    }
}

/// Same as [`try_get_import_settings`], but returns opaque import settings.
pub async fn try_get_import_settings_opaque<T>(
    resource_path: &Path,
    io: &dyn ResourceIo,
) -> Option<Box<dyn BaseImportOptions>>
where
    T: ImportOptions,
{
    try_get_import_settings::<T>(resource_path, io)
        .await
        .map(|options| Box::new(options) as Box<dyn BaseImportOptions>)
}
