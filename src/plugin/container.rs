use crate::{
    core::{
        pool::{Handle, Pool},
        uuid::Uuid,
        visitor::{VisitError, Visitor},
    },
    plugin::{DynamicPlugin, PluginContext, PluginDefinition},
    scene::{
        base::{deserialize_script, serialize_script},
        node::Node,
        Scene,
    },
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

pub struct PluginInstanceData {
    pub id: Uuid,
    pub data: Vec<u8>,
    pub script_instances: Vec<ScriptInstanceData>,
}

pub struct ScriptInstanceData {
    pub scene: Handle<Scene>,
    pub node: Handle<Node>,
    pub data: Vec<u8>,
}

fn serialize_plugin(plugin: &mut DynamicPlugin) -> Result<Vec<u8>, VisitError> {
    let mut visitor = Visitor::new();
    plugin.visit("PluginData", &mut visitor)?;
    visitor.save_binary_to_vec()
}

// Serializes state of a plugin and respective scripts and then unloads the plugin. Serialized
// state will be used to restore data of the plugin (and scripts) after reloading.
fn unload_plugin(mut plugin: DynamicPlugin, context: &mut PluginContext) -> PluginInstanceData {
    let plugin_data = match serialize_plugin(&mut plugin) {
        Ok(data) => data,
        Err(err) => {
            Log::err(format!(
                "Failed to serialize plugin {:?} data! Plugin state won't be restored! Reason: {:?}",
                plugin.lib_path, err,
            ));

            // Set plugin data to empty memory block, deserialization will fail because of this
            // but it is acceptable.
            Vec::new()
        }
    };

    // Serialize and destroy every script instance.
    let mut script_instances = Vec::new();
    for (scene_handle, scene) in context.scenes.pair_iter_mut() {
        for (node_handle, node) in scene.graph.pair_iter_mut() {
            if let Some(script) = node.script.as_ref() {
                if script.plugin_uuid() == plugin.id() {
                    let script = node.script.take().unwrap();

                    match serialize_script(&script) {
                        Ok(data) => script_instances.push(ScriptInstanceData {
                            scene: scene_handle,
                            node: node_handle,
                            data,
                        }),
                        Err(err) => Log::err(format!(
                            "Failed to serialize script instance of type {}\
                            on node {} in {} scene! The state of the script won't be restored!\
                            Reason: {:?}",
                            script.id(),
                            node_handle,
                            scene_handle,
                            err
                        )),
                    }
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

    PluginInstanceData {
        id: plugin.id(),
        script_instances,
        data: plugin_data,
    }
}

impl PluginContainer {
    pub fn new() -> Self {
        Self {
            plugins: Default::default(),
            watcher: None,
        }
    }

    /// Attempts to load plugins specified in `plugins.ron` file in current working directory. Its
    /// main purpose is to load plugins at startup of your app.
    ///
    /// # Panic
    ///
    /// The method will panic if there are any plugins loaded!
    pub fn load(&mut self, context: &mut PluginContext) {
        // There must be no loaded plugins at this point. If it panics, then you probably want to
        // use `reload` instead.
        assert_eq!(self.plugins.alive_count(), 0);

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

    /// Attempts to reload all currently loaded plugins while preserving entire state of the plugins
    /// and respective scripts.
    ///
    /// # Usage
    ///
    /// Due to specifics of dynamic library loading, the method must be used in pair with
    /// [`PluginContainer::clear`]. At first you call [`PluginContainer::clear`], and save the
    /// returned value, at this moment all plugins will be unloaded and will allow you to to
    /// rebuild/replace/modify/etc. your plugins. The final step must be a call of this
    /// method with the value returned on [`PluginContainer::clear`].
    ///
    /// # Panic
    ///
    /// The method will panic if there are any plugins loaded!
    pub fn reload(&mut self, context: &mut PluginContext, instances: Vec<PluginInstanceData>) {
        self.load(context);

        Log::info(format!(
            "Trying to restore state for {} plugin instances...",
            instances.len()
        ));

        for instance_data in instances {
            if let Some(plugin) = self
                .plugins
                .iter_mut()
                .find(|plugin| plugin.id() == instance_data.id)
            {
                // Try to restore plugin state first.
                match Visitor::load_from_memory(instance_data.data) {
                    Ok(mut visitor) => {
                        Log::verify(plugin.visit("PluginData", &mut visitor));
                    }
                    Err(err) => Log::err(format!(
                        "Failed to deserialize plugin {} state! Reason: {:?}",
                        plugin.id(),
                        err
                    )),
                };

                // Try to restore scripts.
                for script_instance_data in instance_data.script_instances {
                    if let Some(scene) = context.scenes.try_get_mut(script_instance_data.scene) {
                        if let Some(node) = scene.graph.try_get_mut(script_instance_data.node) {
                            match deserialize_script(
                                script_instance_data.data,
                                context.serialization_context,
                            ) {
                                Ok(script) => {
                                    node.script = Some(script);
                                }
                                Err(err) => Log::err(format!(
                                    "Failed to restore script instance for node {} in scene {}.\
                                    Reason: {:?}",
                                    script_instance_data.node, script_instance_data.scene, err
                                )),
                            }
                        } else {
                            Log::err(format!(
                                "Failed to restore script instance for node {} in scene {}.\
                                Reason: no such node!",
                                script_instance_data.node, script_instance_data.scene
                            ))
                        }
                    } else {
                        Log::err(format!(
                            "Failed to restore script instance for node {} in scene {}.\
                            Reason: no such scene!",
                            script_instance_data.node, script_instance_data.scene
                        ))
                    }
                }
            }
        }
    }

    pub fn add(&mut self, wrapper: DynamicPlugin) -> Handle<DynamicPlugin> {
        self.plugins.spawn(wrapper)
    }

    #[must_use]
    pub fn free(
        &mut self,
        handle: Handle<DynamicPlugin>,
        context: &mut PluginContext,
    ) -> PluginInstanceData {
        unload_plugin(self.plugins.free(handle), context)
    }

    #[must_use]
    pub fn clear(&mut self, context: &mut PluginContext) -> Vec<PluginInstanceData> {
        let mut instances = Vec::new();

        let plugin_count = self.plugins.alive_count();

        Log::info(format!("Unloading {} plugins...", plugin_count));

        for i in 0..self.plugins.get_capacity() {
            if self.plugins.at(i).is_some() {
                let handle = self.plugins.handle_from_index(i);
                instances.push(unload_plugin(self.plugins.free(handle), context));
            }
        }

        Log::info(format!(
            "{} plugins were unloaded successfully!",
            plugin_count
        ));

        instances
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
