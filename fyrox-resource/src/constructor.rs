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

//! A module for creating resources by their UUIDs. It is used to make resource system type-agnostic
//! yet serializable/deserializable. Type UUID is saved together with resource state and used later
//! on deserialization to create a default instance of corresponding resource.

use crate::{
    core::{parking_lot::Mutex, uuid::Uuid, SafeLock, TypeUuidProvider},
    ResourceData,
};
use fxhash::FxHashMap;

/// A simple type alias for boxed resource constructor.
pub struct ResourceDataConstructor {
    /// Type name of the resource, produced by this constructor.
    pub type_name: String,
    /// Boxed callback, that is able to produce a resource in the default state.
    pub callback: Box<dyn FnMut() -> Box<dyn ResourceData> + Send>,
}

impl ResourceDataConstructor {
    /// Creates a new resource instance in the default state.
    pub fn create_instance(&mut self) -> Box<dyn ResourceData> {
        (self.callback)()
    }
}

/// A special container that is able to create resources by their type UUID.
#[derive(Default)]
pub struct ResourceConstructorContainer {
    /// Map of `Type UUID -> Constructor`
    pub map: Mutex<FxHashMap<Uuid, ResourceDataConstructor>>,
}

impl ResourceConstructorContainer {
    /// Creates default resource data constructor container.
    pub fn new() -> Self {
        ResourceConstructorContainer::default()
    }

    /// Adds new type constructor for a given type and return previous constructor for the type
    /// (if any).
    pub fn add<T>(&self)
    where
        T: ResourceData + Default + TypeUuidProvider,
    {
        let previous = self.map.safe_lock().insert(
            <T as TypeUuidProvider>::type_uuid(),
            ResourceDataConstructor {
                callback: Box::new(|| Box::<T>::default()),
                type_name: std::any::type_name::<T>().to_owned(),
            },
        );

        assert!(previous.is_none());
    }

    /// Adds custom type constructor.
    pub fn add_custom(&self, type_uuid: Uuid, constructor: ResourceDataConstructor) {
        self.map.safe_lock().insert(type_uuid, constructor);
    }

    /// Unregisters type constructor.
    pub fn remove(&self, type_uuid: Uuid) {
        self.map.safe_lock().remove(&type_uuid);
    }

    /// Makes an attempt to create a resource data using provided type UUID. It may fail if there is no
    /// resource data constructor for specified type UUID.
    pub fn try_create(&self, type_uuid: &Uuid) -> Option<Box<dyn ResourceData>> {
        self.map
            .safe_lock()
            .get_mut(type_uuid)
            .map(|c| c.create_instance())
    }

    /// Returns total amount of constructors.
    pub fn len(&self) -> usize {
        self.map.safe_lock().len()
    }

    /// Returns true if the container is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod test {
    use fyrox_core::reflect::prelude::*;
    use fyrox_core::visitor::{Visit, VisitResult, Visitor};
    use std::error::Error;
    use std::path::Path;

    use super::*;

    #[derive(Debug, Default, Clone, Reflect, Visit)]
    struct Stub {}

    impl ResourceData for Stub {
        fn type_uuid(&self) -> Uuid {
            Uuid::default()
        }

        fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
            Err("Saving is not supported!".to_string().into())
        }

        fn can_be_saved(&self) -> bool {
            false
        }

        fn try_clone_box(&self) -> Option<Box<dyn ResourceData>> {
            Some(Box::new(self.clone()))
        }
    }

    impl TypeUuidProvider for Stub {
        fn type_uuid() -> Uuid {
            Uuid::default()
        }
    }

    #[test]
    fn resource_constructor_container_new() {
        let c = ResourceConstructorContainer::new();

        assert_eq!(c.len(), 0);

        c.add::<Stub>();
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn resource_constructor_container_add_custom() {
        let c = ResourceConstructorContainer::new();

        assert!(c.is_empty());

        c.add_custom(
            Uuid::default(),
            ResourceDataConstructor {
                callback: Box::new(|| Box::<Stub>::default()),
                type_name: std::any::type_name::<Stub>().to_owned(),
            },
        );
        assert_eq!(c.len(), 1);

        c.remove(Uuid::default());
        assert!(c.is_empty());
    }

    #[test]
    fn resource_constructor_container_try_create() {
        let c = ResourceConstructorContainer::new();
        c.add::<Stub>();

        let res = c.try_create(&Uuid::default());
        assert!(res.is_some());
    }
}
