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

//! Dynamic plugins with hot-reloading ability.

use crate::plugin::Plugin;
use std::ffi::OsStr;

/// Dynamic plugin, that is loaded from a dynamic library. Usually it is used for hot reloading,
/// it is strongly advised not to use it in production builds, because it is slower than statically
/// linked plugins and it could be unsafe if different compiler versions are used.
pub struct DynamicPlugin {
    pub(super) plugin: Box<dyn Plugin>,
    // Keep the library loaded.
    // Must be last!
    #[allow(dead_code)]
    #[cfg(any(unix, windows))]
    lib: libloading::Library,
}

#[cfg(any(unix, windows))]
type PluginEntryPoint = fn() -> Box<dyn Plugin>;

impl DynamicPlugin {
    /// Tries to load a plugin from a dynamic library (*.dll on Windows, *.so on Unix).
    pub fn load<P>(#[allow(unused_variables)] path: P) -> Result<Self, String>
    where
        P: AsRef<OsStr>,
    {
        #[cfg(any(unix, windows))]
        unsafe {
            let lib = libloading::Library::new(path).map_err(|e| e.to_string())?;

            let entry = lib
                .get::<PluginEntryPoint>("fyrox_plugin".as_bytes())
                .map_err(|e| e.to_string())?;

            Ok(Self {
                plugin: entry(),
                lib,
            })
        }

        #[cfg(not(any(unix, windows)))]
        {
            panic!("Unsupported platform!")
        }
    }

    /// Return a reference to the plugin interface of the dynamic plugin.
    pub fn plugin(&self) -> &dyn Plugin {
        &*self.plugin
    }

    /// Return a reference to the plugin interface of the dynamic plugin.
    pub(crate) fn plugin_mut(&mut self) -> &mut dyn Plugin {
        &mut *self.plugin
    }
}
