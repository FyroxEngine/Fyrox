use crate::{
    core::pool::{Handle, Pool},
    plugin::{DynamicPlugin, PluginContext},
    utils::{log::Log, watcher::FileSystemWatcher},
};
use notify::DebouncedEvent;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

pub struct PluginContainer {
    pub(crate) plugins: Pool<DynamicPlugin>,
    watcher: Option<FileSystemWatcher>,
}

fn unload_plugin(mut plugin: DynamicPlugin, context: &mut PluginContext) {
    plugin.on_unload(context);
}

impl PluginContainer {
    pub fn new() -> Self {
        Self {
            plugins: Default::default(),
            watcher: None,
        }
    }

    pub fn rescan(&mut self, context: &mut PluginContext) {
        self.clear(context);

        Log::info("Looking for plugins recursively from working directory...".to_owned());

        let mut libs = HashMap::<OsString, PathBuf>::new();
        for dir in walkdir::WalkDir::new(".").into_iter().flatten() {
            let path = dir.path();
            if let (Some(file_name), Some(extension)) = (path.file_name(), path.extension()) {
                if let Some(file_name_str) = path.file_stem().and_then(|s| s.to_str()) {
                    if !file_name_str.ends_with("_plugin") {
                        continue;
                    }
                } else {
                    continue;
                }

                if extension == "dll" || extension == "dylib" || extension == "so" {
                    if let Some(prev_candidate) = libs.get_mut(file_name) {
                        if let (Ok(new_candidate_modified_time), Ok(prev_candidate_modified_time)) = (
                            prev_candidate.metadata().and_then(|m| m.modified()),
                            path.metadata().and_then(|m| m.modified()),
                        ) {
                            if new_candidate_modified_time > prev_candidate_modified_time {
                                *prev_candidate = path.to_path_buf();
                            }
                        }
                    } else {
                        libs.insert(file_name.to_os_string(), path.to_path_buf());
                    }
                }
            }
        }

        for path in libs.values() {
            match DynamicPlugin::try_load(path) {
                Ok(mut plugin) => {
                    plugin.on_init(context);
                    let _ = self.plugins.spawn(plugin);

                    Log::info(format!(
                        "Plugin {} was loaded successfully!",
                        path.display()
                    ))
                }
                Err(e) => Log::err(format!(
                    "Unable to load plugin from {}. Reason: {:?}",
                    path.display(),
                    e
                )),
            }
        }

        Log::info(format!(
            "{} plugins were loaded successfully!",
            self.plugins.alive_count()
        ));
    }

    pub fn add(&mut self, wrapper: DynamicPlugin) -> Handle<DynamicPlugin> {
        self.plugins.spawn(wrapper)
    }

    pub fn free(&mut self, handle: Handle<DynamicPlugin>, context: &mut PluginContext) {
        unload_plugin(self.plugins.free(handle), context);
    }

    pub fn clear(&mut self, context: &mut PluginContext) {
        let plugin_count = self.plugins.alive_count();

        Log::info(format!("Unloading {} plugins...", plugin_count));

        for i in 0..self.plugins.get_capacity() {
            if self.plugins.at(i).is_some() {
                let handle = self.plugins.handle_from_index(i);
                unload_plugin(self.plugins.free(handle), context);
            }
        }

        Log::info(format!(
            "{} plugins were unloaded successfully!",
            plugin_count
        ));
    }

    pub fn set_watcher(&mut self, watcher: Option<FileSystemWatcher>) {
        self.watcher = watcher;
    }

    pub fn handle_fs_events(&mut self, context: &mut PluginContext) {
        if let Some(watcher) = self.watcher.as_ref() {
            while let Some(DebouncedEvent::Write(path)) = watcher.try_get_event() {
                'plugin_loop: for i in 0..self.plugins.get_capacity() {
                    if self.plugins.at(i).map_or(false, |p| p.lib_path == path) {
                        let handle = self.plugins.handle_from_index(i);

                        // Unload old plugin first.
                        unload_plugin(self.plugins.free(handle), context);

                        // Try to load new plugin.
                        match DynamicPlugin::try_load(path.as_os_str()) {
                            Ok(mut dynamic_plugin) => {
                                dynamic_plugin.on_init(context);
                                // Put plugin at its previous position to keep update order deterministic
                                // This must never fail, because index is guaranteed to be free.
                                if self.plugins.spawn_at(i, dynamic_plugin).is_err() {
                                    panic!("Unable to preserve plugin location at {}!", i)
                                }
                            }
                            Err(e) => Log::err(format!(
                                "Failed to re-load plugin {}! Reason: {:?}",
                                path.display(),
                                e
                            )),
                        }

                        break 'plugin_loop;
                    }
                }
            }
        }
    }
}
