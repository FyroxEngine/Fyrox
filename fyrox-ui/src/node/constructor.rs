//! A special container that is able to create widgets by their type UUID.

use crate::{
    core::{parking_lot::Mutex, uuid::Uuid, TypeUuidProvider},
    Control, UiNode,
};
use fxhash::FxHashMap;

/// A simple type alias for boxed widget constructor.
pub type WidgetConstructor = Box<dyn FnMut() -> UiNode + Send>;

/// A special container that is able to create widgets by their type UUID.
#[derive(Default)]
pub struct WidgetConstructorContainer {
    map: Mutex<FxHashMap<Uuid, WidgetConstructor>>,
}

impl WidgetConstructorContainer {
    /// Creates default widget constructor container with constructors for built-in widgets.
    pub fn new() -> Self {
        let container = WidgetConstructorContainer::default();

        container
    }

    /// Adds new type constructor for a given type and return previous constructor for the type
    /// (if any).
    pub fn add<T>(&self)
    where
        T: TypeUuidProvider + Control + Default,
    {
        let previous = self
            .map
            .lock()
            .insert(T::type_uuid(), Box::new(|| UiNode::new(T::default())));

        assert!(previous.is_none());
    }

    /// Adds custom type constructor.
    pub fn add_custom(&self, type_uuid: Uuid, constructor: WidgetConstructor) {
        self.map.lock().insert(type_uuid, constructor);
    }

    /// Unregisters type constructor.
    pub fn remove(&self, type_uuid: Uuid) {
        self.map.lock().remove(&type_uuid);
    }

    /// Makes an attempt to create a widget using provided type UUID. It may fail if there is no
    /// widget constructor for specified type UUID.
    pub fn try_create(&self, type_uuid: &Uuid) -> Option<UiNode> {
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
