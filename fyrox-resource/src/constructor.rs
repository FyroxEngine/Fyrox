use crate::{
    core::{parking_lot::Mutex, uuid::Uuid},
    ResourceData,
};
use fxhash::FxHashMap;
use fyrox_core::TypeUuidProvider;

/// A simple type alias for boxed node constructor.
pub type ResourceDataConstructor = Box<dyn FnMut() -> Box<dyn ResourceData> + Send>;

/// A special container that is able to create nodes by their type UUID.
#[derive(Default)]
pub struct ResourceConstructorContainer {
    map: Mutex<FxHashMap<Uuid, ResourceDataConstructor>>,
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
        let previous = self.map.lock().insert(
            <T as TypeUuidProvider>::type_uuid(),
            Box::new(|| Box::new(T::default())),
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
