use crate::{
    core::{inspect::Inspect, uuid::Uuid, visitor::Visit},
    engine::Engine,
};
use fxhash::FxHashMap;
use libloading::{Library, Symbol};
use std::{
    ffi::{OsStr, OsString},
    ops::{Deref, DerefMut},
    path::Path,
};

pub trait GameTrait: Visit + Inspect {
    fn on_init(&mut self, engine: &mut Engine);

    fn script_definition_storage(&self) -> &ScriptDefinitionStorage;
}

pub type EntryPoint = extern "C" fn() -> Box<Box<dyn GameTrait>>;

pub struct Game {
    entry: Box<dyn GameTrait>,

    lib_path: OsString,

    // Must be last to be dropped last!
    #[allow(dead_code)]
    library: Library,
}

impl Deref for Game {
    type Target = dyn GameTrait;

    fn deref(&self) -> &Self::Target {
        &*self.entry
    }
}

impl DerefMut for Game {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.entry
    }
}

impl Game {
    pub fn try_load<P: AsRef<OsStr>>(path: P) -> Result<Self, libloading::Error> {
        unsafe {
            let library = libloading::Library::new(path)?;

            let fyrox_main: Symbol<EntryPoint> = library.get(b"fyrox_main\0")?;

            let entry: Box<Box<dyn GameTrait>> = fyrox_main();

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

pub trait Script: Visit + Inspect {
    fn on_init(&mut self);

    fn type_uuid(&self) -> Uuid;
}

pub struct ScriptDefinition {
    pub name: String,
    pub type_uuid: Uuid,
    pub constructor: Box<dyn FnMut() -> Box<dyn Script>>,
}

#[derive(Default)]
pub struct ScriptDefinitionStorage {
    map: FxHashMap<Uuid, ScriptDefinition>,
}

impl ScriptDefinitionStorage {
    pub fn new() -> Self {
        Self::default()
    }
}
