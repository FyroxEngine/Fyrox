//! Dynamic plugins with hot-reloading ability.

use crate::plugin::Plugin;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// Dynamic plugin, that is loaded from a dynamic library. Usually it is used for hot reloading,
/// it is strongly advised not to use it in production builds, because it is slower than statically
/// linked plugins and it could be unsafe if different compiler versions are used.
pub struct DynamicPlugin {
    pub(super) plugin: Box<dyn Plugin>,
    path: PathBuf,
    // Keep the library loaded.
    // Must be last!
    #[allow(dead_code)]
    lib: libloading::Library,
}

type PluginEntryPoint = fn() -> Box<dyn Plugin>;

impl DynamicPlugin {
    /// Tries to load a plugin from a dynamic library (*.dll on Windows, *.so on Unix).
    pub fn load<P>(path: P) -> Result<Self, String>
    where
        P: AsRef<OsStr>,
    {
        unsafe {
            let lib = libloading::Library::new(path.as_ref()).map_err(|e| e.to_string())?;

            let entry = lib
                .get::<PluginEntryPoint>("fyrox_plugin".as_bytes())
                .map_err(|e| e.to_string())?;

            Ok(Self {
                plugin: entry(),
                path: PathBuf::from(path.as_ref()),
                lib,
            })
        }
    }

    /// Returns the path from which the plugin was loaded from.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
