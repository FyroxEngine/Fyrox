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

use crate::{
    core::{reflect::prelude::*, uuid::Uuid, visitor::prelude::*},
    manager::ResourceManager,
    ResourceData, ResourceLoadError,
};
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

/// A source of a resource uuid.
#[derive(Debug, Reflect, Clone, PartialEq, Eq)]
pub enum ResourcePath {
    /// Explicit path to a file in the file system.
    Explicit(PathBuf),
    /// Implicit path, uses the resource registry to obtain the explicit path in the file system,
    /// that is associated with the given uuid.
    Implicit(Uuid),
}

impl Default for ResourcePath {
    fn default() -> Self {
        Self::Explicit(Default::default())
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
#[derive(Debug, Reflect)]
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
        path: ResourcePath,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A resource path, it is stored only to be able to reload the resources that failed to
        /// load previously.
        path: ResourcePath,
        /// An error. This wrapped in Option only to be Default_ed.
        error: LoadError,
    },
    /// Actual resource data when it is fully loaded.
    Ok(Box<dyn ResourceData>),
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

impl Visit for ResourceState {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        if visitor.is_reading() {
            let mut type_uuid = Uuid::default();
            type_uuid.visit("TypeUuid", visitor)?;

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
                instance.visit(name, visitor)?;
                *self = Self::Ok(instance);
                Ok(())
            } else {
                Err(VisitError::User(format!(
                    "There's no constructor registered for type {type_uuid}!"
                )))
            }
        } else if let Self::Ok(instance) = self {
            instance.visit(name, visitor)?;
            Ok(())
        } else {
            // Do not save other variants, because they're needed only for runtime purposes.
            Ok(())
        }
    }
}

impl ResourceState {
    /// Creates new resource in pending state.
    #[inline]
    pub fn new_pending() -> Self {
        Self::Pending {
            wakers: Default::default(),
            path: Default::default(),
        }
    }

    /// Creates new resource in error state.
    #[inline]
    pub fn new_load_error(path: ResourcePath, error: LoadError) -> Self {
        Self::LoadError { path, error }
    }

    /// Creates new resource in ok (resource with data) state.
    #[inline]
    pub fn new_ok<T: ResourceData>(data: T) -> Self {
        Self::Ok(Box::new(data))
    }

    /// Checks whether the resource is still loading or not.
    pub fn is_loading(&self) -> bool {
        matches!(self, ResourceState::Pending { .. })
    }

    /// Switches the internal state of the resource to [`ResourceState::Pending`].
    pub fn switch_to_pending_state(&mut self) {
        *self = ResourceState::Pending {
            wakers: Default::default(),
            path: Default::default(),
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
            unreachable!()
        };

        *self = state;

        for waker in wakers.0 {
            waker.wake();
        }
    }

    /// Changes internal state to [`ResourceState::Ok`]
    pub fn commit_ok<T: ResourceData>(&mut self, data: T) {
        self.commit(ResourceState::Ok(Box::new(data)))
    }

    /// Changes internal state to [`ResourceState::LoadError`].
    pub fn commit_error<E: ResourceLoadError>(&mut self, path: ResourcePath, error: E) {
        self.commit(ResourceState::LoadError {
            path,
            error: LoadError::new(error),
        })
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

    #[derive(Debug, Default, Reflect, Visit)]
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
    }

    impl TypeUuidProvider for Stub {
        fn type_uuid() -> Uuid {
            Uuid::default()
        }
    }

    #[test]
    fn resource_state_new_pending() {
        let state = ResourceState::new_pending();

        assert!(matches!(state, ResourceState::Pending { wakers: _ }));
        assert!(state.is_loading());
    }

    #[test]
    fn resource_state_new_load_error() {
        let state = ResourceState::new_load_error(Default::default());

        assert!(matches!(state, ResourceState::LoadError { error: _ }));
        assert!(!state.is_loading());
    }

    #[test]
    fn resource_state_new_ok() {
        let state = ResourceState::new_ok(Stub {});
        assert!(matches!(state, ResourceState::Ok(_)));
        assert!(!state.is_loading());
    }

    #[test]
    fn resource_state_switch_to_pending_state() {
        // from Ok
        let mut state = ResourceState::new_ok(Stub {});
        state.switch_to_pending_state();

        assert!(matches!(state, ResourceState::Pending { wakers: _ }));

        // from LoadError
        let mut state = ResourceState::new_load_error(Default::default());
        state.switch_to_pending_state();

        assert!(matches!(state, ResourceState::Pending { wakers: _ }));

        // from Pending
        let mut state = ResourceState::new_pending();
        state.switch_to_pending_state();

        assert!(matches!(state, ResourceState::Pending { wakers: _ }));
    }

    #[test]
    fn visit_for_resource_state() {
        // Visit Pending
        let mut state = ResourceState::new_pending();
        let mut visitor = Visitor::default();

        assert!(state.visit("name", &mut visitor).is_ok());

        // Visit LoadError
        let mut state = ResourceState::new_load_error(Default::default());
        let mut visitor = Visitor::default();

        assert!(state.visit("name", &mut visitor).is_ok());

        // Visit Ok
        let mut state = ResourceState::new_ok(Stub {});
        let mut visitor = Visitor::default();

        assert!(state.visit("name", &mut visitor).is_ok());
    }
}
