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

use crate::{
    core::{
        log::Log,
        notify::{self, EventKind, RecursiveMode, Watcher},
    },
    plugin::Plugin,
};
use crate::core::notify::RecommendedWatcher;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    ffi::OsStr
};

use crate::plugin::AbstractDynamicPlugin;

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


/// Implementation of DynamicPluginTrait that [re]loads Rust code from Rust dylib .
pub struct DyLibPlugin {
    /// Dynamic plugin state.
    state: DynamicPluginState,
    /// Target path of the library of the plugin.
    lib_path: PathBuf,
    /// Path to the source file, that is emitted by the compiler. If hot reloading is enabled,
    /// this library will be cloned to `lib_path` and loaded. This is needed, because usually
    /// OS locks the library and it is not possible to overwrite it while it is loaded in a process.  
    source_lib_path: PathBuf,
    /// Optional file system watcher, that is configured to watch the source library and re-load
    /// the plugin if the source library has changed. If the watcher is `None`, then hot reloading
    /// is disabled.
    watcher: Option<RecommendedWatcher>,
    /// A flag, that tells the engine that the plugin needs to be reloaded. Usually the engine
    /// will do that at the end of the update tick.
    need_reload: Arc<AtomicBool>,
}

impl DyLibPlugin {
	
    /// Tries to create a new dynamic plugin. This method attempts to load a dynamic library by the
    /// given path and searches for `fyrox_plugin` function. This function is called to create a
    /// plugin instance. This method will fail if there's no dynamic library at the given path or
    /// the `fyrox_plugin` function is not found.
    ///
    /// # Hot reloading
    ///
    /// This method can enable hot reloading for the plugin, by setting `reload_when_changed` parameter
    /// to `true`. When enabled, the engine will clone the library to implementation-defined path
    /// and load it. It will setup file system watcher to receive changes from the OS and reload
    /// the plugin.
    pub fn new<P>(
        path: P,
        reload_when_changed: bool,
        use_relative_paths: bool,
    ) -> Result<Self, String>
    where
        P: AsRef<Path> + 'static,
    {
        let source_lib_path = if use_relative_paths {
            let exe_folder = std::env::current_exe()
                .map_err(|e| e.to_string())?
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();

            exe_folder.join(path.as_ref())
        } else {
            path.as_ref().to_path_buf()
        };

        let plugin = if reload_when_changed {
            // Make sure each process will its own copy of the module. This is needed to prevent
            // issues when there are two or more running processes and a library of the plugin
            // changes. If the library is present in one instance in both (or more) processes, then
            // it is impossible to replace it on disk. To prevent this, we need to add a suffix with
            // executable name.
            let mut suffix = std::env::current_exe()
                .ok()
                .and_then(|p| p.file_stem().map(|s| s.to_owned()))
                .unwrap_or_default();
            suffix.push(".module");
            let lib_path = source_lib_path.with_extension(suffix);
            try_copy_library(&source_lib_path, &lib_path)?;

            let need_reload = Arc::new(AtomicBool::new(false));
            let need_reload_clone = need_reload.clone();
            let source_lib_path_clone = source_lib_path.clone();

            let mut watcher =
                notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                    if let Ok(event) = event {
                        if let EventKind::Modify(_) | EventKind::Create(_) = event.kind {
                            need_reload_clone.store(true, atomic::Ordering::Relaxed);

                            Log::warn(format!(
                                "Plugin {} was changed. Performing hot reloading...",
                                source_lib_path_clone.display()
                            ))
                        }
                    }
                })
                .map_err(|e| e.to_string())?;

            watcher
                .watch(&source_lib_path, RecursiveMode::NonRecursive)
                .map_err(|e| e.to_string())?;

            Log::info(format!(
                "Watching for changes in plugin {:?}...",
                source_lib_path
            ));

            DyLibPlugin {
                state: DynamicPluginState::Loaded(DynamicPlugin::load(lib_path.as_os_str())?),
                lib_path,
                source_lib_path: source_lib_path.clone(),
                watcher: Some(watcher),
                need_reload,
            }
        } else {
            DyLibPlugin {
                state: DynamicPluginState::Loaded(DynamicPlugin::load(
                    source_lib_path.as_os_str(),
                )?),
                lib_path: source_lib_path.clone(),
                source_lib_path: source_lib_path.clone(),
                watcher: None,
                need_reload: Default::default(),
            }
        };
        Ok(plugin)
    }
}

impl AbstractDynamicPlugin for DyLibPlugin {
    fn as_loaded_ref(&self) -> &dyn Plugin {
        &*self.state.as_loaded_ref().plugin
    }

    fn as_loaded_mut(&mut self) -> &mut dyn Plugin {
        &mut *self.state.as_loaded_mut().plugin
    }

    fn is_reload_needed_now(&self) -> bool {
        self.need_reload.load(atomic::Ordering::Relaxed)
    }

    fn display_name(&self) -> String {
        format!("{:?}", self.source_lib_path)
    }

    fn is_loaded(&self) -> bool {
        matches!(self.state, DynamicPluginState::Loaded { .. })
    }

    fn reload(&mut self, fill_and_register: &mut dyn FnMut(&mut dyn Plugin) -> Result<(), String>) -> Result<(), String> {
        // Unload the plugin.
        let DynamicPluginState::Loaded(_) = &mut self.state else {
            return Err("cannot unload non-loaded plugin".to_string());
        };

        self.state = DynamicPluginState::Unloaded;

        Log::info(format!(
            "Plugin {:?} was unloaded successfully!",
            self.source_lib_path
        ));

        // Replace the module.
        try_copy_library(&self.source_lib_path, &self.lib_path)?;

        Log::info(format!(
            "{:?} plugin's module {} was successfully cloned to {}.",
            self.source_lib_path,
            self.source_lib_path.display(),
            self.lib_path.display()
        ));

        let mut dynamic = DynamicPlugin::load(&self.lib_path)?;

        fill_and_register(dynamic.plugin_mut())?;

        self.state = DynamicPluginState::Loaded(dynamic);

        self.need_reload.store(false, atomic::Ordering::Relaxed);

        Log::info(format!(
            "Plugin {:?} was reloaded successfully!",
            self.source_lib_path
        ));

        Ok(())
    }
}

/// Actual state of a dynamic plugin.
pub enum DynamicPluginState {
    /// Unloaded plugin.
    Unloaded,
    /// Loaded plugin.
    Loaded(DynamicPlugin),
}

impl DynamicPluginState {
    /// Tries to interpret the state as [`Self::Loaded`], panics if the plugin is unloaded.
    pub fn as_loaded_ref(&self) -> &DynamicPlugin {
        match self {
            DynamicPluginState::Unloaded => {
                panic!("Cannot obtain a reference to the plugin, because it is unloaded!")
            }
            DynamicPluginState::Loaded(dynamic) => dynamic,
        }
    }

    /// Tries to interpret the state as [`Self::Loaded`], panics if the plugin is unloaded.
    pub fn as_loaded_mut(&mut self) -> &mut DynamicPlugin {
        match self {
            DynamicPluginState::Unloaded => {
                panic!("Cannot obtain a reference to the plugin, because it is unloaded!")
            }
            DynamicPluginState::Loaded(dynamic) => dynamic,
        }
    }
}

fn try_copy_library(source_lib_path: &Path, lib_path: &Path) -> Result<(), String> {
    if let Err(err) = std::fs::copy(source_lib_path, lib_path) {
        // The library could already be copied and loaded, thus cannot be replaced. For
        // example - by the running editor, that also uses hot reloading. Check for matching
        // content, and if does not match, pass the error further.
        let mut src_lib_file = File::open(source_lib_path).map_err(|e| e.to_string())?;
        let mut src_lib_file_content = Vec::new();
        src_lib_file
            .read_to_end(&mut src_lib_file_content)
            .map_err(|e| e.to_string())?;
        let mut lib_file = File::open(lib_path).map_err(|e| e.to_string())?;
        let mut lib_file_content = Vec::new();
        lib_file
            .read_to_end(&mut lib_file_content)
            .map_err(|e| e.to_string())?;
        if src_lib_file_content != lib_file_content {
            return Err(format!(
                "Unable to clone the library {} to {}. It is required, because source \
                        library has {} size, but loaded has {} size and the content does not match. \
                        Exact reason: {:?}",
                source_lib_path.display(),
                lib_path.display(),
                src_lib_file_content.len(),
                lib_file_content.len(),
                err
            ));
        }
    }

    Ok(())
}
