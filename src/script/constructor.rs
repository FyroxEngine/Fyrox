//! A special container that is able to create nodes by their type UUID.

use crate::{
    core::{
        parking_lot::{Mutex, MutexGuard},
        uuid::Uuid,
    },
    scene::node::TypeUuidProvider,
    script::{Script, ScriptTrait},
};
use std::collections::BTreeMap;

pub struct ScriptConstructor {
    /// A simple type alias for boxed node constructor.
    pub constructor: Box<dyn FnMut() -> Script + Send>,

    /// Script name.
    pub name: String,
}

/// A special container that is able to create nodes by their type UUID.
#[derive(Default)]
pub struct ScriptConstructorContainer {
    // BTreeMap allows to have sorted list of constructors.
    map: Mutex<BTreeMap<Uuid, ScriptConstructor>>,
}

impl ScriptConstructorContainer {
    /// Creates default node constructor container with constructors for built-in engine nodes.
    pub fn new() -> Self {
        ScriptConstructorContainer::default()
    }

    /// Adds new type constructor for a given type and return previous constructor for the type
    /// (if any).
    pub fn add<T, N>(&self, name: N) -> Option<ScriptConstructor>
    where
        T: TypeUuidProvider + ScriptTrait + Default,
        N: AsRef<str>,
    {
        self.map.lock().insert(
            T::type_uuid(),
            ScriptConstructor {
                constructor: Box::new(|| Script::new(T::default())),
                name: name.as_ref().to_string(),
            },
        )
    }

    /// Adds custom type constructor.
    pub fn add_custom(&self, type_uuid: Uuid, constructor: ScriptConstructor) {
        self.map.lock().insert(type_uuid, constructor);
    }

    /// Unregisters type constructor.
    pub fn remove(&self, type_uuid: Uuid) {
        self.map.lock().remove(&type_uuid);
    }

    /// Makes an attempt to create a script using provided type UUID. It may fail if there is no
    /// script constructor for specified type UUID.
    pub fn try_create(&self, type_uuid: &Uuid) -> Option<Script> {
        self.map
            .lock()
            .get_mut(type_uuid)
            .map(|c| (c.constructor)())
    }

    /// Returns inner map of script constructors.
    pub fn map(&self) -> MutexGuard<BTreeMap<Uuid, ScriptConstructor>> {
        self.map.lock()
    }
}
