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

/// Script constructor contains all required data and methods to create script instances
/// by their UUIDs. Its is primarily used for serialization needs.
pub struct ScriptConstructor {
    /// Parent plugin UUID.
    pub plugin_uuid: Uuid,

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

    /// Adds new type constructor for a given type.
    ///
    /// # Panic
    ///
    /// The method will panic if there is already a constructor for given type uuid.
    pub fn add<P, T, N>(&self, name: N)
    where
        P: TypeUuidProvider,
        T: TypeUuidProvider + ScriptTrait + Default,
        N: AsRef<str>,
    {
        let old = self.map.lock().insert(
            T::type_uuid(),
            ScriptConstructor {
                plugin_uuid: P::type_uuid(),
                constructor: Box::new(|| Script::new(T::default())),
                name: name.as_ref().to_string(),
            },
        );

        assert!(old.is_none());
    }

    /// Adds custom type constructor.
    ///
    /// # Panic
    ///
    /// The method will panic if there is already a constructor for given type uuid.
    pub fn add_custom(&self, type_uuid: Uuid, constructor: ScriptConstructor) {
        let old = self.map.lock().insert(type_uuid, constructor);

        assert!(old.is_none());
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
