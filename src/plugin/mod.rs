use crate::{
    core::{inspect::Inspect, visitor::Visit},
    engine::resource_manager::ResourceManager,
    gui::UserInterface,
    renderer::Renderer,
    scene::SceneContainer,
    script::ScriptDefinitionStorage,
};
use libloading::{Library, Symbol};
use std::{
    ffi::{OsStr, OsString},
    ops::{Deref, DerefMut},
};

pub mod container;

pub struct PluginContext<'a> {
    pub scenes: &'a mut SceneContainer,
    pub ui: &'a mut UserInterface,
    pub resource_manager: &'a ResourceManager,
    pub renderer: &'a mut Renderer,
    pub dt: f32,
}

pub trait Plugin: Visit + Inspect {
    fn on_init(&mut self, context: &mut PluginContext);

    fn on_unload(&mut self, context: &mut PluginContext);

    fn update(&mut self, context: &mut PluginContext);

    fn script_definition_storage(&self) -> &ScriptDefinitionStorage;
}

pub type EntryPoint = extern "C" fn() -> Box<Box<dyn Plugin>>;

pub struct DynamicPlugin {
    entry: Box<dyn Plugin>,

    lib_path: OsString,

    // Must be last to be dropped last!
    #[allow(dead_code)]
    library: Library,
}

impl Deref for DynamicPlugin {
    type Target = dyn Plugin;

    fn deref(&self) -> &Self::Target {
        &*self.entry
    }
}

impl DerefMut for DynamicPlugin {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.entry
    }
}

impl DynamicPlugin {
    pub fn try_load<P: AsRef<OsStr>>(path: P) -> Result<Self, libloading::Error> {
        unsafe {
            let library = libloading::Library::new(path.as_ref())?;

            let fyrox_main: Symbol<EntryPoint> = library.get(b"fyrox_main\0")?;

            let entry: Box<Box<dyn Plugin>> = fyrox_main();

            Ok(Self {
                library,
                lib_path: path.as_ref().to_os_string(),
                entry: *entry,
            })
        }
    }

    pub fn lib_path(&self) -> &OsStr {
        &self.lib_path
    }
}
