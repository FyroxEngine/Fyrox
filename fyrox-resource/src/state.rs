//! A module that handles resource states.

use crate::{
    core::{
        curve::Curve,
        uuid::Uuid,
        visitor::{prelude::*, RegionGuard},
    },
    manager::ResourceManager,
    ResourceData, ResourceLoadError, CURVE_RESOURCE_UUID, MODEL_RESOURCE_UUID,
    SHADER_RESOURCE_UUID, SOUND_BUFFER_RESOURCE_UUID, TEXTURE_RESOURCE_UUID,
};
use std::{
    borrow::Cow,
    ffi::OsStr,
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
#[derive(Debug)]
pub enum ResourceState {
    /// Resource is loading from external resource or in the queue to load.
    Pending {
        /// A path to load resource from.
        path: PathBuf,
        /// List of wakers to wake future when resource is fully loaded.
        wakers: Vec<Waker>,
        /// Unique type id of the resource.
        type_uuid: Uuid,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: PathBuf,
        /// An error. This wrapped in Option only to be Default_ed.
        error: Option<Arc<dyn ResourceLoadError>>,
        /// Unique type id of the resource.
        type_uuid: Uuid,
    },
    /// Actual resource data when it is fully loaded.
    Ok(Box<dyn ResourceData>),
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
                        error: None,
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
    pub fn new_load_error(
        path: PathBuf,
        error: Option<Arc<dyn ResourceLoadError>>,
        type_uuid: Uuid,
    ) -> Self {
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
        matches!(self, Self::Pending { .. })
    }

    /// Switches the internal state of the resource to [`ResourceState::Pending`].
    pub fn switch_to_pending_state(&mut self) {
        match self {
            Self::LoadError {
                path, type_uuid, ..
            } => {
                *self = Self::Pending {
                    path: std::mem::take(path),
                    wakers: Default::default(),
                    type_uuid: *type_uuid,
                }
            }
            Self::Ok(data) => {
                *self = Self::Pending {
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
            Self::Pending { type_uuid, .. } => *type_uuid,
            Self::LoadError { type_uuid, .. } => *type_uuid,
            Self::Ok(data) => data.type_uuid(),
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
    pub fn path(&self) -> Cow<Path> {
        match self {
            Self::Pending { path, .. } => Cow::Borrowed(path.as_path()),
            Self::LoadError { path, .. } => Cow::Borrowed(path.as_path()),
            Self::Ok(details) => details.path(),
        }
    }

    /// Changes ResourceState::Pending state to ResourceState::Ok(data) with given `data`.
    /// Additionally it wakes all futures.
    #[inline]
    pub fn commit(&mut self, state: Self) {
        assert!(!matches!(state, Self::Pending { .. }));

        let wakers = if let Self::Pending { ref mut wakers, .. } = self {
            std::mem::take(wakers)
        } else {
            unreachable!()
        };

        *self = state;

        for waker in wakers {
            waker.wake();
        }
    }

    /// Changes internal state to [`ResourceState::Ok`]
    pub fn commit_ok<T: ResourceData>(&mut self, data: T) {
        self.commit(Self::Ok(Box::new(data)))
    }

    /// Changes internal state to [`ResourceState::LoadError`].
    pub fn commit_error<E: ResourceLoadError>(&mut self, path: PathBuf, error: E) {
        let type_uuid = self.type_uuid();
        self.commit(Self::LoadError {
            path,
            error: Some(Arc::new(error)),
            type_uuid,
        })
    }
}
