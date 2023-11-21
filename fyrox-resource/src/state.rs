//! A module that handles resource states.

use crate::{
    core::{
        curve::Curve,
        log::Log,
        reflect::prelude::*,
        uuid::Uuid,
        visitor::{prelude::*, RegionGuard},
    },
    manager::ResourceManager,
    ResourceData, ResourceLoadError, CURVE_RESOURCE_UUID, MODEL_RESOURCE_UUID,
    SHADER_RESOURCE_UUID, SOUND_BUFFER_RESOURCE_UUID, TEXTURE_RESOURCE_UUID,
};
use std::{
    ffi::OsStr,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Arc,
    task::Waker,
};

// Heuristic function to guess resource uuid based on inner content of a resource.
fn guess_uuid(region: &mut RegionGuard) -> Uuid {
    assert!(region.is_reading());

    let mut region = region.enter_region("Details").unwrap();

    let mut mip_count = 0u32;
    if mip_count.visit("MipCount", &mut region).is_ok() {
        return TEXTURE_RESOURCE_UUID;
    }

    let mut curve = Curve::default();
    if curve.visit("Curve", &mut region).is_ok() {
        return CURVE_RESOURCE_UUID;
    }

    let mut id = 0u32;
    if id.visit("Id", &mut region).is_ok() {
        return SOUND_BUFFER_RESOURCE_UUID;
    }

    let mut path = PathBuf::new();
    if path.visit("Path", &mut region).is_ok() {
        let ext = path.extension().unwrap_or_default().to_ascii_lowercase();
        if ext == OsStr::new("rgs") || ext == OsStr::new("fbx") {
            return MODEL_RESOURCE_UUID;
        } else if ext == OsStr::new("shader")
            || path == OsStr::new("Standard")
            || path == OsStr::new("StandardTwoSides")
            || path == OsStr::new("StandardTerrain")
        {
            return SHADER_RESOURCE_UUID;
        }
    }

    Default::default()
}

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
        /// A path to load resource from.        
        path: PathBuf,
        /// List of wakers to wake future when resource is fully loaded.
        wakers: WakersList,
        /// Unique type id of the resource.
        type_uuid: Uuid,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: PathBuf,
        /// An error. This wrapped in Option only to be Default_ed.
        error: LoadError,
        /// Unique type id of the resource.
        type_uuid: Uuid,
    },
    /// Actual resource data when it is fully loaded.
    Ok(Box<dyn ResourceData>),
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::LoadError {
            error: Default::default(),
            path: Default::default(),
            type_uuid: Default::default(),
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
        let mut region = visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", &mut region)?;

        match id {
            0 => {
                if region.is_reading() {
                    let mut path = PathBuf::new();
                    path.visit("Path", &mut region)?;

                    let mut type_uuid = Uuid::default();
                    let _ = type_uuid.visit("TypeUuid", &mut region);

                    *self = Self::Pending {
                        path,
                        wakers: Default::default(),
                        type_uuid,
                    };

                    Ok(())
                } else if let Self::Pending {
                    path, type_uuid, ..
                } = self
                {
                    let _ = type_uuid.visit("TypeUuid", &mut region);
                    path.visit("Path", &mut region)
                } else {
                    Err(VisitError::User("Enum variant mismatch!".to_string()))
                }
            }
            1 => {
                if region.is_reading() {
                    let mut path = PathBuf::new();
                    path.visit("Path", &mut region)?;

                    let mut type_uuid = Uuid::default();
                    let _ = type_uuid.visit("TypeUuid", &mut region);

                    *self = Self::LoadError {
                        path,
                        error: Default::default(),
                        type_uuid,
                    };

                    Ok(())
                } else if let Self::LoadError {
                    path, type_uuid, ..
                } = self
                {
                    let _ = type_uuid.visit("TypeUuid", &mut region);
                    path.visit("Path", &mut region)
                } else {
                    Err(VisitError::User("Enum variant mismatch!".to_string()))
                }
            }
            2 => {
                if region.is_reading() {
                    let mut type_uuid = Uuid::default();
                    if type_uuid.visit("TypeUuid", &mut region).is_err() {
                        // We might be reading the old version, try to guess an actual type uuid by
                        // the inner content of the resource data.
                        type_uuid = guess_uuid(&mut region);
                    }

                    let resource_manager = region.blackboard.get::<ResourceManager>().expect(
                        "Resource data constructor container must be \
                provided when serializing resources!",
                    );
                    let resource_manager_state = resource_manager.state();

                    if let Some(mut instance) = resource_manager_state
                        .constructors_container
                        .try_create(&type_uuid)
                    {
                        drop(resource_manager_state);
                        instance.visit("Details", &mut region)?;
                        *self = Self::Ok(instance);
                        Ok(())
                    } else {
                        Err(VisitError::User(format!(
                            "There's no constructor registered for type {type_uuid}!"
                        )))
                    }
                } else if let Self::Ok(instance) = self {
                    let mut type_uuid = instance.type_uuid();
                    type_uuid.visit("TypeUuid", &mut region)?;
                    instance.visit("Details", &mut region)?;
                    Ok(())
                } else {
                    Err(VisitError::User("Enum variant mismatch!".to_string()))
                }
            }
            _ => Err(VisitError::User(format!("Invalid resource state id {id}!"))),
        }
    }
}

impl ResourceState {
    /// Creates new resource in pending state.
    #[inline]
    pub fn new_pending(path: PathBuf, type_uuid: Uuid) -> Self {
        Self::Pending {
            path,
            wakers: Default::default(),
            type_uuid,
        }
    }

    /// Creates new resource in error state.
    #[inline]
    pub fn new_load_error(path: PathBuf, error: LoadError, type_uuid: Uuid) -> Self {
        Self::LoadError {
            path,
            error,
            type_uuid,
        }
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
        match self {
            ResourceState::LoadError {
                path, type_uuid, ..
            } => {
                *self = ResourceState::Pending {
                    path: std::mem::take(path),
                    wakers: Default::default(),
                    type_uuid: *type_uuid,
                }
            }
            ResourceState::Ok(data) => {
                *self = ResourceState::Pending {
                    path: data.path().to_path_buf(),
                    wakers: Default::default(),
                    type_uuid: data.type_uuid(),
                }
            }
            _ => (),
        }
    }

    /// Returns unique type id of the resource.
    pub fn type_uuid(&self) -> Uuid {
        match self {
            ResourceState::Pending { type_uuid, .. } => *type_uuid,
            ResourceState::LoadError { type_uuid, .. } => *type_uuid,
            ResourceState::Ok(data) => data.type_uuid(),
        }
    }

    #[inline]
    fn id(&self) -> u32 {
        match self {
            Self::Pending { .. } => 0,
            Self::LoadError { .. } => 1,
            Self::Ok(_) => 2,
        }
    }

    /// Returns a path to the resource source.
    #[inline]
    pub fn path(&self) -> &Path {
        match self {
            Self::Pending { path, .. } => path,
            Self::LoadError { path, .. } => path,
            Self::Ok(details) => details.path(),
        }
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
    pub fn commit_error<E: ResourceLoadError>(&mut self, path: PathBuf, error: E) {
        let type_uuid = self.type_uuid();

        Log::err(format!(
            "An error occurred while loading {} resource. Reason: {:?}",
            path.display(),
            error
        ));

        self.commit(ResourceState::LoadError {
            path,
            error: LoadError::new(error),
            type_uuid,
        })
    }
}

#[cfg(test)]
mod test {

    use fyrox_core::{
        reflect::{FieldInfo, Reflect},
        TypeUuidProvider,
    };

    use super::*;

    #[derive(Debug, Default, Reflect, Visit)]
    struct Stub {}

    impl ResourceData for Stub {
        fn path(&self) -> &std::path::Path {
            Path::new("")
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
            Uuid::default()
        }

        fn is_procedural(&self) -> bool {
            unimplemented!()
        }
    }

    impl TypeUuidProvider for Stub {
        fn type_uuid() -> Uuid {
            Uuid::default()
        }
    }

    #[test]
    fn resource_state_new_pending() {
        let path = PathBuf::from("foo.txt");
        let type_uuid = Uuid::default();
        let state = ResourceState::new_pending(path.clone(), type_uuid);

        assert!(matches!(
            state,
            ResourceState::Pending {
                path: _,
                wakers: _,
                type_uuid: _
            }
        ));
        assert_eq!(state.path(), path);
        assert_eq!(state.type_uuid(), type_uuid);
        assert_eq!(state.id(), 0);
        assert!(state.is_loading());
    }

    #[test]
    fn resource_state_new_load_error() {
        let path = PathBuf::from("foo.txt");
        let type_uuid = Uuid::default();
        let state = ResourceState::new_load_error(path.clone(), Default::default(), type_uuid);

        assert!(matches!(
            state,
            ResourceState::LoadError {
                path: _,
                error: _,
                type_uuid: _
            }
        ));
        assert_eq!(state.path(), path);
        assert_eq!(state.type_uuid(), type_uuid);
        assert_eq!(state.id(), 1);
        assert!(!state.is_loading());
    }

    #[test]
    fn resource_state_new_ok() {
        let state = ResourceState::new_ok(Stub {});

        assert!(matches!(state, ResourceState::Ok(_)));
        assert_eq!(state.path(), Path::new(""));
        assert_eq!(state.type_uuid(), Uuid::default());
        assert_eq!(state.id(), 2);
        assert!(!state.is_loading());
    }

    #[test]
    fn resource_state_switch_to_pending_state() {
        // from Ok
        let mut state = ResourceState::new_ok(Stub {});
        state.switch_to_pending_state();

        assert!(matches!(
            state,
            ResourceState::Pending {
                path: _,
                wakers: _,
                type_uuid: _
            }
        ));
        assert_eq!(state.path(), Path::new(""));
        assert_eq!(state.type_uuid(), Uuid::default());

        // from LoadError
        let path = PathBuf::from("foo.txt");
        let type_uuid = Uuid::default();
        let mut state = ResourceState::new_load_error(path.clone(), Default::default(), type_uuid);
        state.switch_to_pending_state();

        assert!(matches!(
            state,
            ResourceState::Pending {
                path: _,
                wakers: _,
                type_uuid: _
            }
        ));
        assert_eq!(state.path(), path);
        assert_eq!(state.type_uuid(), type_uuid);

        // from Pending
        let path = PathBuf::from("foo.txt");
        let type_uuid = Uuid::default();
        let mut state = ResourceState::new_pending(path.clone(), type_uuid);
        state.switch_to_pending_state();

        assert!(matches!(
            state,
            ResourceState::Pending {
                path: _,
                wakers: _,
                type_uuid: _
            }
        ));
        assert_eq!(state.path(), path);
        assert_eq!(state.type_uuid(), type_uuid);
    }

    #[test]
    fn visit_for_resource_state() {
        // Visit Pending
        let mut state = ResourceState::new_pending(PathBuf::from("foo.txt"), Uuid::default());
        let mut visitor = Visitor::default();

        assert!(state.visit("name", &mut visitor).is_ok());

        // Visit LoadError
        let mut state = ResourceState::new_load_error(
            PathBuf::from("foo.txt"),
            Default::default(),
            Uuid::default(),
        );
        let mut visitor = Visitor::default();

        assert!(state.visit("name", &mut visitor).is_ok());

        // Visit Ok
        let mut state = ResourceState::new_ok(Stub {});
        let mut visitor = Visitor::default();

        assert!(state.visit("name", &mut visitor).is_ok());
    }

    #[test]
    fn stub_path() {
        let s = Stub {};
        assert_eq!(s.path(), std::borrow::Cow::Borrowed(Path::new("")));
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
    fn stub_set_type_uuid() {
        let s = Stub {};
        assert_eq!(s.type_uuid(), Uuid::default());
    }
}
