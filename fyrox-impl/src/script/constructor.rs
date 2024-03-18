//! A special container that is able to create nodes by their type UUID.

use crate::{
    core::{
        parking_lot::{Mutex, MutexGuard},
        uuid::Uuid,
        TypeUuidProvider,
    },
    script::{Script, ScriptTrait},
};
use std::any::{Any, TypeId};
use std::collections::BTreeMap;

/// Script constructor contains all required data and methods to create script instances
/// by their UUIDs. Its is primarily used for serialization needs.
pub struct ScriptConstructor {
    /// A simple type alias for boxed node constructor.
    pub constructor: Box<dyn FnMut() -> Script + Send>,

    /// Script name.
    pub name: String,

    /// Script source path.
    pub source_path: String,

    /// A type of the source of the script constructor.
    pub source_type_id: TypeId,
}

/// A special container that is able to create nodes by their type UUID.
pub struct ScriptConstructorContainer {
    pub(crate) context_type_id: Mutex<TypeId>,
    // BTreeMap allows to have sorted list of constructors.
    map: Mutex<BTreeMap<Uuid, ScriptConstructor>>,
}

impl Default for ScriptConstructorContainer {
    fn default() -> Self {
        Self {
            context_type_id: Mutex::new(().type_id()),
            map: Default::default(),
        }
    }
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
    pub fn add<T>(&self, name: &str) -> &Self
    where
        T: TypeUuidProvider + ScriptTrait + Default,
    {
        let old = self.map.lock().insert(
            T::type_uuid(),
            ScriptConstructor {
                constructor: Box::new(|| Script::new(T::default())),
                name: name.to_owned(),
                source_path: T::source_path().to_owned(),
                source_type_id: *self.context_type_id.lock(),
            },
        );

        assert!(old.is_none());

        self
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
