//! A module for creating resources by their UUIDs. It is used to make resource system type-agnostic
//! yet serializable/deserializable. Type UUID is saved together with resource state and used later
//! on deserialization to create a default instance of corresponding resource.

use crate::{
    core::{parking_lot::Mutex, uuid::Uuid, TypeUuidProvider},
    ResourceData,
};
use fxhash::FxHashMap;

/// A simple type alias for boxed resource constructor.
pub type ResourceDataConstructor = Box<dyn FnMut() -> Box<dyn ResourceData> + Send>;

/// A special container that is able to create resources by their type UUID.
#[derive(Default)]
pub struct ResourceConstructorContainer {
    map: Mutex<FxHashMap<Uuid, ResourceDataConstructor>>,
}

impl ResourceConstructorContainer {
    /// Creates default resource data constructor container.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds new type constructor for a given type and return previous constructor for the type
    /// (if any).
    pub fn add<T>(&self)
    where
        T: ResourceData + Default + TypeUuidProvider,
    {
        let previous = self.map.lock().insert(
            <T as TypeUuidProvider>::type_uuid(),
            Box::new(|| Box::<T>::default()),
        );

        assert!(previous.is_none());
    }

    /// Adds custom type constructor.
    pub fn add_custom(&self, type_uuid: Uuid, constructor: ResourceDataConstructor) {
        self.map.lock().insert(type_uuid, constructor);
    }

    /// Unregisters type constructor.
    pub fn remove(&self, type_uuid: Uuid) {
        self.map.lock().remove(&type_uuid);
    }

    /// Makes an attempt to create a resource data using provided type UUID. It may fail if there is no
    /// resource data constructor for specified type UUID.
    pub fn try_create(&self, type_uuid: &Uuid) -> Option<Box<dyn ResourceData>> {
        self.map.lock().get_mut(type_uuid).map(|c| (c)())
    }

    /// Returns total amount of constructors.
    pub fn len(&self) -> usize {
        self.map.lock().len()
    }

    /// Returns true if the container is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use fyrox_core::reflect::prelude::*;
    use fyrox_core::visitor::{Visit, VisitResult, Visitor};

    use super::*;

    #[derive(Debug, Default, Reflect, Visit)]
    struct Stub {}

    impl ResourceData for Stub {
        fn path(&self) -> std::borrow::Cow<std::path::Path> {
            unimplemented!()
        }

        fn set_path(&mut self, _path: std::path::PathBuf) {
            unimplemented!()
        }

        fn as_any(&self) -> &dyn std::any::Any {
            unimplemented!()
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            unimplemented!()
        }

        fn type_uuid(&self) -> Uuid {
            unimplemented!()
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

        c.add_custom(Uuid::default(), Box::new(|| Box::new(Stub {})));
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

    #[test]
    #[should_panic]
    fn stub_path() {
        let s = Stub {};
        s.path();
    }

    #[test]
    #[should_panic]
    fn stub_set_path() {
        let mut s = Stub {};
        s.set_path(PathBuf::new());
    }

    #[test]
    #[should_panic]
    fn stub_set_as_any() {
        let s = Stub {};
        ResourceData::as_any(&s);
    }

    #[test]
    #[should_panic]
    fn stub_set_as_any_mut() {
        let mut s = Stub {};
        ResourceData::as_any_mut(&mut s);
        s.type_uuid();
    }

    #[test]
    #[should_panic]
    fn stub_set_type_uuid() {
        let s = Stub {};
        s.type_uuid();
    }
}
