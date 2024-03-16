//! Dynamic plugins with hot-reloading ability.

use crate::plugin::PluginConstructor;
use std::ffi::OsStr;

pub struct DynamicPlugin {
    pub constructor: Box<dyn PluginConstructor>,
    // Keep the library loaded.
    // Must be last!
    #[allow(dead_code)]
    lib: libloading::Library,
}

pub type PluginEntryPoint = fn() -> Box<dyn PluginConstructor>;

impl DynamicPlugin {
    pub fn load<P>(path: P) -> Result<Self, String>
    where
        P: AsRef<OsStr>,
    {
        unsafe {
            let lib = libloading::Library::new(path).map_err(|e| e.to_string())?;

            let entry = lib
                .get::<PluginEntryPoint>("fyrox_plugin".as_bytes())
                .map_err(|e| e.to_string())?;

            Ok(Self {
                constructor: entry(),
                lib,
            })
        }
    }
}
