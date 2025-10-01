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

//! A module that handles resource states.

use crate::untyped::ResourceKind;
use crate::{
    core::{reflect::prelude::*, uuid::Uuid, visitor::prelude::*},
    manager::ResourceManager,
    ResourceData, ResourceLoadError, TypedResourceData,
};
use fyrox_core::reflect::ReflectHandle;
use fyrox_core::warn;
use std::any::{Any, TypeId};
use std::path::PathBuf;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
    task::Waker,
};

#[doc(hidden)]
#[derive(Reflect, Debug, Default, Clone)]
#[reflect(hide_all)]
pub struct WakersList(Vec<Waker>);

impl Deref for WakersList {
    type Target = Vec<Waker>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl WakersList {
    pub fn add_waker(&mut self, cx_waker: &Waker) {
        if let Some(pos) = self.iter().position(|waker| waker.will_wake(cx_waker)) {
            self[pos].clone_from(cx_waker);
        } else {
            self.push(cx_waker.clone())
        }
    }
}

impl DerefMut for WakersList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Arbitrary loading error, that could be optionally be empty.  
#[derive(Reflect, Debug, Clone, Default)]
#[reflect(hide_all)]
pub struct LoadError(pub Option<Arc<dyn ResourceLoadError>>);

impl LoadError {
    /// Creates new loading error from a value of the given type.
    pub fn new<T: ResourceLoadError>(value: T) -> Self {
        Self(Some(Arc::new(value)))
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ResourceDataWrapper(pub Box<dyn ResourceData>);

impl Reflect for ResourceDataWrapper {
    fn source_path() -> &'static str
    where
        Self: Sized,
    {
        file!()
    }

    fn derived_types() -> &'static [TypeId]
    where
        Self: Sized,
    {
        &[]
    }

    fn try_clone_box(&self) -> Option<Box<dyn Reflect>> {
        Reflect::try_clone_box(&*self.0)
    }

    fn query_derived_types(&self) -> &'static [TypeId] {
        self.deref().query_derived_types()
    }

    fn type_name(&self) -> &'static str {
        self.deref().type_name()
    }

    fn doc(&self) -> &'static str {
        self.deref().doc()
    }

    fn fields_ref(&self, func: &mut dyn FnMut(&[FieldRef])) {
        self.deref().fields_ref(func)
    }

    fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [FieldMut])) {
        self.deref_mut().fields_mut(func)
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self, func: &mut dyn FnMut(&dyn Any)) {
        self.deref().as_any(func)
    }

    fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn Any)) {
        self.deref_mut().as_any_mut(func)
    }

    fn as_reflect(&self, func: &mut dyn FnMut(&dyn Reflect)) {
        self.deref().as_reflect(func)
    }

    fn as_reflect_mut(&mut self, func: &mut dyn FnMut(&mut dyn Reflect)) {
        self.deref_mut().as_reflect_mut(func)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<Box<dyn Reflect>, Box<dyn Reflect>> {
        self.deref_mut().set(value)
    }

    fn assembly_name(&self) -> &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn type_assembly_name() -> &'static str
    where
        Self: Sized,
    {
        env!("CARGO_PKG_NAME")
    }

    fn set_field(
        &mut self,
        field: &str,
        value: Box<dyn Reflect>,
        func: &mut dyn FnMut(Result<Box<dyn Reflect>, SetFieldError>),
    ) {
        self.deref_mut().set_field(field, value, func)
    }

    fn field(&self, name: &str, func: &mut dyn FnMut(Option<&dyn Reflect>)) {
        self.deref().field(name, func)
    }

    fn field_mut(&mut self, name: &str, func: &mut dyn FnMut(Option<&mut dyn Reflect>)) {
        self.deref_mut().field_mut(name, func)
    }

    fn as_array(&self, func: &mut dyn FnMut(Option<&dyn ReflectArray>)) {
        self.deref().as_array(func)
    }

    fn as_array_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectArray>)) {
        self.deref_mut().as_array_mut(func)
    }

    fn as_list(&self, func: &mut dyn FnMut(Option<&dyn ReflectList>)) {
        self.deref().as_list(func)
    }

    fn as_list_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectList>)) {
        self.deref_mut().as_list_mut(func)
    }

    fn as_inheritable_variable(
        &self,
        func: &mut dyn FnMut(Option<&dyn ReflectInheritableVariable>),
    ) {
        self.deref().as_inheritable_variable(func)
    }

    fn as_inheritable_variable_mut(
        &mut self,
        func: &mut dyn FnMut(Option<&mut dyn ReflectInheritableVariable>),
    ) {
        self.deref_mut().as_inheritable_variable_mut(func)
    }

    fn as_hash_map(&self, func: &mut dyn FnMut(Option<&dyn ReflectHashMap>)) {
        self.deref().as_hash_map(func)
    }

    fn as_hash_map_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectHashMap>)) {
        self.deref_mut().as_hash_map_mut(func)
    }

    fn as_handle(&self, func: &mut dyn FnMut(Option<&dyn ReflectHandle>)) {
        self.deref().as_handle(func)
    }

    fn as_handle_mut(&mut self, func: &mut dyn FnMut(Option<&mut dyn ReflectHandle>)) {
        self.deref_mut().as_handle_mut(func)
    }
}

impl Deref for ResourceDataWrapper {
    type Target = dyn ResourceData;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl DerefMut for ResourceDataWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl Clone for ResourceDataWrapper {
    fn clone(&self) -> Self {
        Self(ResourceData::try_clone_box(&*self.0).unwrap())
    }
}

/// Resource could be in three possible states (a small state machine):
///
/// 1. Pending - it is loading or queued for loading.
/// 2. LoadError - an error has occurred during the load.
/// 3. Ok - resource is fully loaded and ready to use.
///
/// ## Why is it so complex?
///
/// Short answer: asynchronous loading.
/// Long answer: when you're loading a scene, you expect it to be loaded as fast as possible, use
/// all available power of the CPU. To achieve that, each resource ideally should be loaded on
/// separate core of the CPU, but since this is asynchronous, we must be able to track the state
/// of the resource.
///
/// ## Path
///
/// Resources do not store their paths to respective files in the file system, instead resource only
/// stores their unique identifiers (UUID). Use [`crate::registry::ResourceRegistry`] to get a path
/// associated with the resource uuid.
///
/// ## UUID
///
/// Resource UUID is available only if the resource is fully loaded. This is because there's no way
/// to get the UUID earlier: the UUID is stored in a metadata file which exists only if the resource
/// is present. It is somewhat possible to get a UUID when a resource is failed to load, but not in
/// 100% cases.
#[derive(Debug, Clone, Reflect)]
pub enum ResourceState {
    /// Resource is loading from external resource or in the queue to load.
    Pending {
        /// List of wakers to wake future when resource is fully loaded.
        wakers: WakersList,
        /// A resource path (explicit or implicit). It is used at the loading stage to get a
        /// real path in the file system. Since resource registry loading is async (especially
        /// on WASM), it is impossible to fetch the uuid by path immediately. Instead, the resource
        /// system offloads this task to resource loading tasks, which are able to wait until the
        /// registry is fully loaded.
        path: PathBuf,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A resource path, it is stored only to be able to reload the resources that failed to
        /// load previously.
        path: PathBuf,
        /// An error. This wrapped in Option only to be Default_ed.
        error: LoadError,
    },
    /// Actual resource data when it is fully loaded.
    Ok {
        /// Unique id of the resource.
        resource_uuid: Uuid,
        /// Actual data of the resource.
        data: ResourceDataWrapper,
    },
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::LoadError {
            path: Default::default(),
            error: Default::default(),
        }
    }
}

impl Drop for ResourceState {
    fn drop(&mut self) {
        if let ResourceState::Pending { wakers, .. } = self {
            assert_eq!(wakers.len(), 0);
        }
    }
}

impl ResourceState {
    pub(crate) fn visit(
        &mut self,
        kind: ResourceKind,
        name: &str,
        visitor: &mut Visitor,
    ) -> VisitResult {
        if visitor.is_reading() {
            let mut type_uuid = Uuid::default();
            type_uuid.visit("TypeUuid", visitor)?;

            let mut resource_uuid = Uuid::default();
            if resource_uuid.visit("ResourceUuid", visitor).is_err() {
                warn!(
                    "A resource of type {type_uuid} has no uuid! It looks like a resource in \
              the old format; trying to read it..."
                );
            }

            let resource_manager = visitor.blackboard.get::<ResourceManager>().expect(
                "Resource data constructor container must be \
                provided when serializing resources!",
            );
            let resource_manager_state = resource_manager.state();

            if let Some(mut instance) = resource_manager_state
                .constructors_container
                .try_create(&type_uuid)
            {
                drop(resource_manager_state);

                if kind == ResourceKind::Embedded {
                    instance.visit(name, visitor)?;
                }

                *self = Self::Ok {
                    resource_uuid,
                    data: ResourceDataWrapper(instance),
                };
            } else {
                return Err(VisitError::User(format!(
                    "There's no constructor registered for type {type_uuid}!"
                )));
            }

            Ok(())
        } else if let Self::Ok {
            resource_uuid,
            data,
        } = self
        {
            resource_uuid.visit("ResourceUuid", visitor)?;

            let mut type_uuid = data.type_uuid();
            type_uuid.visit("TypeUuid", visitor)?;

            if kind == ResourceKind::Embedded {
                data.visit(name, visitor)?;
            }

            Ok(())
        } else {
            // Do not save other variants, because they're needed only for runtime purposes.
            Ok(())
        }
    }

    /// Creates new resource in pending state.
    #[inline]
    pub fn new_pending(path: PathBuf) -> Self {
        Self::Pending {
            wakers: Default::default(),
            path,
        }
    }

    /// Creates new resource in error state.
    #[inline]
    pub fn new_load_error(path: PathBuf, error: LoadError) -> Self {
        Self::LoadError { path, error }
    }

    /// Creates new resource in [`ResourceState::Ok`] state.
    #[inline]
    pub fn new_ok<T: ResourceData>(resource_uuid: Uuid, data: T) -> Self {
        Self::Ok {
            resource_uuid,
            data: ResourceDataWrapper(Box::new(data)),
        }
    }

    /// Creates a new resource in [`ResourceState::Ok`] state using arbitrary data.
    #[inline]
    pub fn new_ok_untyped(resource_uuid: Uuid, data: Box<dyn ResourceData>) -> Self {
        Self::Ok {
            resource_uuid,
            data: ResourceDataWrapper(data),
        }
    }

    /// Tries to get a resource uuid. The uuid is available only for resource in [`ResourceState::Ok`]
    /// state.
    #[inline]
    pub fn resource_uuid(&self) -> Option<Uuid> {
        match self {
            ResourceState::Ok { resource_uuid, .. } => Some(*resource_uuid),
            _ => None,
        }
    }

    /// Checks whether the resource is still loading or not.
    pub fn is_loading(&self) -> bool {
        matches!(self, ResourceState::Pending { .. })
    }

    /// Switches the internal state of the resource to [`ResourceState::Pending`].
    pub fn switch_to_pending_state(&mut self, path: PathBuf) {
        *self = ResourceState::Pending {
            wakers: Default::default(),
            path,
        };
    }

    /// Changes ResourceState::Pending state to ResourceState::Ok(data) with given `data`.
    /// Additionally it wakes all futures.
    #[inline]
    pub fn commit(&mut self, state: ResourceState) {
        assert!(!matches!(state, ResourceState::Pending { .. }));

        let wakers = if let ResourceState::Pending { ref mut wakers, .. } = self {
            std::mem::take(wakers)
        } else {
            Default::default()
        };

        *self = state;

        for waker in wakers.0 {
            waker.wake();
        }
    }

    /// Changes internal state to [`ResourceState::Ok`]
    pub fn commit_ok<T: ResourceData>(&mut self, resource_uuid: Uuid, data: T) {
        self.commit(ResourceState::Ok {
            resource_uuid,
            data: ResourceDataWrapper(Box::new(data)),
        })
    }

    /// Changes internal state to [`ResourceState::LoadError`].
    pub fn commit_error<E: ResourceLoadError>(&mut self, path: PathBuf, error: E) {
        self.commit(ResourceState::LoadError {
            path,
            error: LoadError::new(error),
        })
    }

    /// Tries to get the resource data. Will fail if the resource is not in [`ResourceState::Ok`].
    pub fn data_ref(&self) -> Option<&ResourceDataWrapper> {
        match self {
            ResourceState::Pending { .. } | ResourceState::LoadError { .. } => None,
            ResourceState::Ok { data, .. } => Some(data),
        }
    }

    /// Tries to get the resource data. Will fail if the resource is not in [`ResourceState::Ok`].
    pub fn data_mut(&mut self) -> Option<&mut ResourceDataWrapper> {
        match self {
            ResourceState::Pending { .. } | ResourceState::LoadError { .. } => None,
            ResourceState::Ok { data, .. } => Some(data),
        }
    }

    /// Tries to get the resource data of the given type. Will fail if the resource is not in
    /// [`ResourceState::Ok`].
    pub fn data_ref_of_type<T: TypedResourceData>(&self) -> Option<&T> {
        match self {
            ResourceState::Pending { .. } | ResourceState::LoadError { .. } => None,
            ResourceState::Ok { data, .. } => (&**data as &dyn Any).downcast_ref::<T>(),
        }
    }

    /// Tries to get the resource data of the given type. Will fail if the resource is not in
    /// [`ResourceState::Ok`].
    pub fn data_mut_of_type<T: TypedResourceData>(&mut self) -> Option<&mut T> {
        match self {
            ResourceState::Pending { .. } | ResourceState::LoadError { .. } => None,
            ResourceState::Ok { data, .. } => (&mut **data as &mut dyn Any).downcast_mut::<T>(),
        }
    }
}

#[cfg(test)]
mod test {
    use fyrox_core::{
        reflect::{FieldRef, Reflect},
        TypeUuidProvider,
    };
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
    fn resource_state_new_pending() {
        let state = ResourceState::new_pending(Default::default());

        assert!(matches!(state, ResourceState::Pending { .. }));
        assert!(state.is_loading());
    }

    #[test]
    fn resource_state_new_load_error() {
        let state = ResourceState::new_load_error(Default::default(), Default::default());

        assert!(matches!(state, ResourceState::LoadError { .. }));
        assert!(!state.is_loading());
    }

    #[test]
    fn resource_state_new_ok() {
        let uuid = Uuid::new_v4();
        let state = ResourceState::new_ok(uuid, Stub {});
        assert!(matches!(state, ResourceState::Ok { .. }));
        assert!(!state.is_loading());
    }

    #[test]
    fn resource_state_switch_to_pending_state() {
        // from Ok
        let mut state = ResourceState::new_ok(Uuid::new_v4(), Stub {});
        state.switch_to_pending_state(Default::default());

        assert!(matches!(state, ResourceState::Pending { .. }));

        // from LoadError
        let mut state = ResourceState::new_load_error(Default::default(), Default::default());
        state.switch_to_pending_state(Default::default());

        assert!(matches!(state, ResourceState::Pending { .. }));

        // from Pending
        let mut state = ResourceState::new_pending(Default::default());
        state.switch_to_pending_state(Default::default());

        assert!(matches!(state, ResourceState::Pending { .. }));
    }

    #[test]
    fn visit_for_resource_state() {
        // Visit Pending
        let mut state = ResourceState::new_pending(Default::default());
        let mut visitor = Visitor::default();

        assert!(state
            .visit(ResourceKind::External, "name", &mut visitor)
            .is_ok());

        // Visit LoadError
        let mut state = ResourceState::new_load_error(Default::default(), Default::default());
        let mut visitor = Visitor::default();

        assert!(state
            .visit(ResourceKind::External, "name", &mut visitor)
            .is_ok());

        // Visit Ok
        let mut state = ResourceState::new_ok(Uuid::new_v4(), Stub {});
        let mut visitor = Visitor::default();

        assert!(state
            .visit(ResourceKind::External, "name", &mut visitor)
            .is_ok());
    }
}
