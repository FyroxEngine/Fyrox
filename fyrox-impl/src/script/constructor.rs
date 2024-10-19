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

//! A special container that is able to create nodes by their type UUID.

use crate::{
    core::{
        parking_lot::{Mutex, MutexGuard},
        uuid::Uuid,
        TypeUuidProvider,
    },
    script::{Script, ScriptTrait},
};
use std::collections::BTreeMap;

/// Script constructor contains all required data and methods to create script instances
/// by their UUIDs. Its is primarily used for serialization needs.
pub struct ScriptConstructor {
    /// A simple type alias for boxed node constructor.
    pub constructor: Box<dyn FnMut() -> Script + Send>,

    /// Script name.
    pub name: String,

    /// Script source path.
    pub source_path: &'static str,

    /// A name of the assembly this script constructor belongs to.
    pub assembly_name: &'static str,
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
    pub fn add<T>(&self, name: &str) -> &Self
    where
        T: TypeUuidProvider + ScriptTrait + Default,
    {
        let old = self.map.lock().insert(
            T::type_uuid(),
            ScriptConstructor {
                constructor: Box::new(|| Script::new(T::default())),
                name: name.to_owned(),
                source_path: T::source_path(),
                assembly_name: T::type_assembly_name(),
            },
        );

        assert!(old.is_none());

        self
    }

    /// Adds custom type constructor.
    pub fn add_custom(
        &self,
        type_uuid: Uuid,
        constructor: ScriptConstructor,
    ) -> Result<(), String> {
        let mut map = self.map.lock();
        if let Some(old) = map.get(&type_uuid) {
            return Err(format!(
                "cannot add {} ({}) because its uuid is already used by {} ({})",
                constructor.name, constructor.assembly_name, old.name, old.assembly_name
            ));
        }
        map.insert(type_uuid, constructor);
        Ok(())
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
