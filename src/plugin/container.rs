use crate::{
    core::pool::{Handle, Pool},
    plugin::{DynamicPlugin, PluginContext, PluginDefinition},
    utils::{log::Log, watcher::FileSystemWatcher},
};
use notify::DebouncedEvent;
use serde::Deserialize;
use std::{error::Error, fs::File, path::Path};

pub struct PluginContainer {
    pub(crate) plugins: Pool<DynamicPlugin>,
    watcher: Option<FileSystemWatcher>,
}

#[derive(Deserialize)]
pub struct PluginContainerDefinition {
    plugins: Vec<PluginDefinition>,
}

impl PluginContainerDefinition {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        Ok(ron::de::from_reader(file)?)
    }
}

fn unload_plugin(mut plugin: DynamicPlugin, context: &mut PluginContext) {
    // Destroy every script instance.
    for scene in context.scenes.iter_mut() {
        for node in scene.graph.linear_iter_mut() {
            if let Some(script) = node.script.as_ref() {
                if script.plugin_uuid() == plugin.id() {
                    node.script = None;
                }
            }
        }
    }

    // Unregister script constructors.
    context
        .serialization_context
        .script_constructors
        .map()
        .retain(|_, constructor| constructor.plugin_uuid != plugin.id());

    plugin.on_unload(context);
}

impl PluginContainer {
    pub fn new() -> Self {
        Self {
            plugins: Default::default(),
            watcher: None,
        }
    }

    pub fn reload(&mut self, context: &mut PluginContext) {
        self.clear(context);

        Log::info("Looking for `plugins.ron` in the working directory...".to_owned());

        match PluginContainerDefinition::from_file("plugins.ron") {
            Ok(definition) => {
                for plugin_definition in definition.plugins {
                    match DynamicPlugin::try_load(&plugin_definition.path) {
                        Ok(mut plugin) => {
                            plugin.on_init(context);
                            let _ = self.plugins.spawn(plugin);

                            Log::info(format!(
                                "Plugin {} was loaded from {} successfully!",
                                plugin_definition.name,
                                plugin_definition.path.display()
                            ))
                        }
                        Err(e) => Log::err(format!(
                            "Unable to load {} plugin from {}. Reason: {:?}",
                            plugin_definition.name,
                            plugin_definition.path.display(),
                            e
                        )),
                    }
                }

                Log::info(format!(
                    "{} plugins were loaded successfully!",
                    self.plugins.alive_count()
                ));
            }
            Err(e) => Log::err(format!(
                "Unable to load plugin definition container. Reason {:?}",
                e
            )),
        }
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

    pub fn iter(&self) -> impl Iterator<Item = &DynamicPlugin> {
        self.plugins.iter()
    }
}
