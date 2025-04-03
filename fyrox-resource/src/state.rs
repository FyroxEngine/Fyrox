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
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
    task::Waker,
};

#[doc(hidden)]
#[derive(Reflect, Debug, Default)]
#[reflect(hide_all)]
pub struct WakersList(Vec<Waker>);

impl Deref for WakersList {
    type Target = Vec<Waker>;

    fn deref(&self) -> &Self::Target {
        &self.0
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

/// Resource could be in three possible states:
/// 1. Pending - it is loading.
/// 2. LoadError - an error has occurred during the load.
/// 3. Ok - resource is fully loaded and ready to use.
///
/// Why it is so complex?
/// Short answer: asynchronous loading.
/// Long answer: when you loading a scene you expect it to be loaded as fast as
/// possible, use all available power of the CPU. To achieve that each resource
/// ideally should be loaded on separate core of the CPU, but since this is
/// asynchronous, we must have the ability to track the state of the resource.
#[derive(Debug, Reflect)]
pub enum ResourceState {
    /// Resource is loading from external resource or in the queue to load.
    Pending {
        /// List of wakers to wake future when resource is fully loaded.
        wakers: WakersList,
    },
    /// An error has occurred during the load.
    LoadError {
        /// An error. This wrapped in Option only to be Default_ed.
        error: LoadError,
    },
    /// Actual resource data when it is fully loaded.
    Ok(Box<dyn ResourceData>),
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::LoadError {
            error: Default::default(),
        }
    }
}

impl Drop for ResourceState {
    fn drop(&mut self) {
        if let Self::Pending { wakers, .. } = self {
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
        }
    }

    /// Creates new resource in error state.
    #[inline]
    pub fn new_load_error(error: LoadError) -> Self {
        Self::LoadError { error }
    }

    /// Creates new resource in ok (resource with data) state.
    #[inline]
    pub fn new_ok<T: ResourceData>(data: T) -> Self {
        Self::Ok(Box::new(data))
    }

    /// Checks whether the resource is still loading or not.
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Pending { .. })
    }

    /// Switches the internal state of the resource to [`ResourceState::Pending`].
    pub fn switch_to_pending_state(&mut self) {
        *self = Self::Pending {
            wakers: Default::default(),
        };
    }

    /// Changes ResourceState::Pending state to ResourceState::Ok(data) with given `data`.
    /// Additionally it wakes all futures.
    #[inline]
    pub fn commit(&mut self, state: Self) {
        assert!(!matches!(state, Self::Pending { .. }));

        let wakers = if let Self::Pending { ref mut wakers } = self {
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
        self.commit(Self::Ok(Box::new(data)))
    }

    /// Changes internal state to [`ResourceState::LoadError`].
    pub fn commit_error<E: ResourceLoadError>(&mut self, error: E) {
        self.commit(Self::LoadError {
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
