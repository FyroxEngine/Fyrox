//! Resource import options common traits.

use crate::core::{append_extension, io};
use ron::ser::PrettyConfig;
use serde::{de::DeserializeOwned, Serialize};
use std::{fs::File, path::Path};

/// A trait for resource import options. It provides generic functionality shared over all types of import options.
pub trait ImportOptions: Serialize + DeserializeOwned + Default + Clone {
    /// Saves import options into a specified file.
    fn save(&self, path: &Path) -> bool {
        if let Ok(file) = File::create(path) {
            if ron::ser::to_writer_pretty(file, self, PrettyConfig::default()).is_ok() {
                return true;
            }
        }
        false
    }
}

/// Tries to load import settings for a resource. It is not part of ImportOptions trait because
/// `async fn` is not yet supported for traits.
pub async fn try_get_import_settings<T>(resource_path: &Path) -> Option<T>
where
    T: ImportOptions,
{
    let settings_path = append_extension(resource_path, "options");

    match io::load_file(&settings_path).await {
        Ok(bytes) => match ron::de::from_bytes::<T>(&bytes) {
            Ok(options) => Some(options),
            Err(e) => {
                // TODO: Use logger when it will be moved to fyrox_core.
                eprintln!(
                    "Malformed options file {} for {} resource! Reason: {:?}",
                    settings_path.display(),
                    resource_path.display(),
                    e
                );

                None
            }
        },
        Err(e) => {
            // TODO: Use logger when it will be moved to fyrox_core.
            eprintln!(
                "Unable to load options file {} for {} resource, fallback to defaults! Reason: {:?}",
                settings_path.display(),
                resource_path.display(),
                e
            );

            None
        }
    }
}
