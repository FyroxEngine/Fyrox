//! Resource management

// #![warn(missing_docs)] TODO

use crate::{
    core::{
        parking_lot::MutexGuard,
        reflect::prelude::*,
        reflect::FieldValue,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    state::ResourceState,
};
use std::{
    any::Any,
    borrow::Cow,
    fmt::{Debug, Formatter},
    future::Future,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::manager::ResourceManager;
use crate::untyped::UntypedResource;
pub use fyrox_core as core;
use fyrox_core::TypeUuidProvider;

pub mod constructor;
pub mod container;
pub mod loader;
pub mod manager;
pub mod options;
pub mod state;
pub mod task;
pub mod untyped;

pub const TEXTURE_RESOURCE_UUID: Uuid = uuid!("02c23a44-55fa-411a-bc39-eb7a5eadf15c");
pub const MODEL_RESOURCE_UUID: Uuid = uuid!("44cd768f-b4ca-4804-a98c-0adf85577ada");
pub const SOUND_BUFFER_RESOURCE_UUID: Uuid = uuid!("f6a077b7-c8ff-4473-a95b-0289441ea9d8");
pub const SHADER_RESOURCE_UUID: Uuid = uuid!("f1346417-b726-492a-b80f-c02096c6c019");
pub const CURVE_RESOURCE_UUID: Uuid = uuid!("f28b949f-28a2-4b68-9089-59c234f58b6b");

/// A trait for resource data.
pub trait ResourceData: 'static + Debug + Visit + Send {
    /// Returns path of resource data.
    fn path(&self) -> Cow<Path>;

    /// Sets new path to resource data.
    fn set_path(&mut self, path: PathBuf);

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_uuid(&self) -> Uuid;
}

/// A trait for resource load error.
pub trait ResourceLoadError: 'static + Debug + Send + Sync {}

impl<T> ResourceLoadError for T where T: 'static + Debug + Send + Sync {}

pub struct ResourceStateGuard<'a, T>
where
    T: ResourceData + TypeUuidProvider,
{
    guard: MutexGuard<'a, ResourceState>,
    phantom: PhantomData<T>,
}

impl<'a, T> ResourceStateGuard<'a, T>
where
    T: ResourceData + TypeUuidProvider,
{
    pub fn get(&mut self) -> ResourceStateRef<'_, T> {
        match &*self.guard {
            ResourceState::Pending {
                path, type_uuid, ..
            } => ResourceStateRef::Pending {
                path,
                type_uuid: *type_uuid,
            },
            ResourceState::LoadError {
                path,
                error,
                type_uuid,
            } => ResourceStateRef::LoadError {
                path,
                error,
                type_uuid: *type_uuid,
            },
            ResourceState::Ok(data) => ResourceStateRef::Ok(
                (&**data as &dyn ResourceData)
                    .as_any()
                    .downcast_ref()
                    .expect("Type mismatch!"),
            ),
        }
    }

    pub fn get_mut(&mut self) -> ResourceStateRefMut<'_, T> {
        match &mut *self.guard {
            ResourceState::Pending {
                path, type_uuid, ..
            } => ResourceStateRefMut::Pending {
                path,
                type_uuid: *type_uuid,
            },
            ResourceState::LoadError {
                path,
                error,
                type_uuid,
            } => ResourceStateRefMut::LoadError {
                path,
                error,
                type_uuid: *type_uuid,
            },
            ResourceState::Ok(data) => ResourceStateRefMut::Ok(
                (&mut **data as &mut dyn ResourceData)
                    .as_any_mut()
                    .downcast_mut()
                    .expect("Type mismatch!"),
            ),
        }
    }
}

#[derive(Debug)]
pub enum ResourceStateRef<'a, T>
where
    T: ResourceData,
{
    /// Resource is loading from external resource or in the queue to load.
    Pending {
        /// A path to load resource from.
        path: &'a PathBuf,
        type_uuid: Uuid,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: &'a PathBuf,
        /// An error.
        error: &'a Option<Arc<dyn ResourceLoadError>>,
        type_uuid: Uuid,
    },
    /// Actual resource data when it is fully loaded.
    Ok(&'a T),
}

#[derive(Debug)]
pub enum ResourceStateRefMut<'a, T> {
    /// Resource is loading from external resource or in the queue to load.
    Pending {
        /// A path to load resource from.
        path: &'a mut PathBuf,
        type_uuid: Uuid,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: &'a mut PathBuf,
        /// An error.
        error: &'a mut Option<Arc<dyn ResourceLoadError>>,
        type_uuid: Uuid,
    },
    /// Actual resource data when it is fully loaded.
    Ok(&'a mut T),
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::LoadError {
            error: None,
            path: Default::default(),
            type_uuid: Default::default(),
        }
    }
}

#[derive(Debug, Reflect)]
#[reflect(hide_all)]
pub struct Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    state: Option<UntypedResource>,
    phantom: PhantomData<T>,
}

impl<T> Visit for Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.state.visit("State", &mut region)?;

        if region.is_reading() {
            // Try to restore the shallow handle.
            let resource_manager = region
                .blackboard
                .get::<ResourceManager>()
                .expect("Resource manager must be available when deserializing resources!");

            let path = self.state.as_ref().unwrap().path();

            // Procedural resources usually have path empty or use it as an id, in this case we need to
            // check if the file actually exists to not mess up procedural resources.
            if path.exists() {
                self.state = Some(
                    resource_manager.request_untyped(path, <T as TypeUuidProvider>::type_uuid()),
                );
            }
        }

        Ok(())
    }
}

impl<T> PartialEq for Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    fn eq(&self, other: &Self) -> bool {
        match (&self.state, &other.state) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T> Eq for Resource<T> where T: ResourceData + TypeUuidProvider {}

impl<T> Hash for Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state.hash(state)
    }
}

impl<T> Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    pub fn new_pending(path: PathBuf) -> Self {
        Self {
            state: Some(UntypedResource::new_pending(
                path,
                <T as TypeUuidProvider>::type_uuid(),
            )),
            phantom: PhantomData,
        }
    }

    pub fn new_ok(data: T) -> Self {
        Self {
            state: Some(UntypedResource::new_ok(data)),
            phantom: PhantomData,
        }
    }

    pub fn new_load_error(path: PathBuf, error: Option<Arc<dyn ResourceLoadError>>) -> Self {
        Self {
            state: Some(UntypedResource::new_load_error(
                path,
                error,
                <T as TypeUuidProvider>::type_uuid(),
            )),
            phantom: PhantomData,
        }
    }

    /// Converts self to internal value.
    #[inline]
    pub fn into_inner(self) -> UntypedResource {
        self.state.unwrap()
    }

    /// Locks internal mutex provides access to the state.
    #[inline]
    pub fn state(&self) -> ResourceStateGuard<'_, T> {
        ResourceStateGuard {
            guard: self.state_inner(),
            phantom: Default::default(),
        }
    }

    /// Tries to lock internal mutex provides access to the state.
    #[inline]
    pub fn try_acquire_state(&self) -> Option<ResourceStateGuard<'_, T>> {
        if let Some(guard) = self.state.as_ref().unwrap().0.try_lock() {
            Some(ResourceStateGuard {
                guard,
                phantom: Default::default(),
            })
        } else {
            None
        }
    }

    fn state_inner(&self) -> MutexGuard<'_, ResourceState> {
        self.state.as_ref().unwrap().0.lock()
    }

    /// Returns true if the resource is still loading.
    pub fn is_loading(&self) -> bool {
        matches!(*self.state_inner(), ResourceState::Pending { .. })
    }

    /// Returns exact amount of users of the resource.
    #[inline]
    pub fn use_count(&self) -> usize {
        self.state.as_ref().unwrap().use_count()
    }

    /// Returns a pointer as numeric value which can be used as a hash.
    #[inline]
    pub fn key(&self) -> usize {
        self.state.as_ref().unwrap().key()
    }

    pub fn path(&self) -> PathBuf {
        self.state.as_ref().unwrap().0.lock().path().to_path_buf()
    }

    /// Allows you to obtain reference to the resource data.
    ///
    /// # Panic
    ///
    /// An attempt to use method result will panic if resource is not loaded yet, or
    /// there was load error. Usually this is ok because normally you'd chain this call
    /// like this `resource.await?.data_ref()`. Every resource implements Future trait
    /// and it returns Result, so if you'll await future then you'll get Result, so
    /// call to `data_ref` will be fine.
    #[inline]
    pub fn data_ref(&self) -> ResourceDataRef<'_, T> {
        ResourceDataRef {
            guard: self.state_inner(),
            phantom: Default::default(),
        }
    }
}

impl<T> Default for Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    #[inline]
    fn default() -> Self {
        Self {
            state: None,
            phantom: Default::default(),
        }
    }
}

impl<T> Clone for Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            phantom: Default::default(),
        }
    }
}

impl<T> From<UntypedResource> for Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    #[inline]
    fn from(state: UntypedResource) -> Self {
        Self {
            state: Some(state),
            phantom: Default::default(),
        }
    }
}

#[allow(clippy::from_over_into)]
impl<T> Into<UntypedResource> for Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    #[inline]
    fn into(self) -> UntypedResource {
        self.state.unwrap()
    }
}

impl<T> Future for Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    type Output = Result<Self, Option<Arc<dyn ResourceLoadError>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.state.as_ref().unwrap().clone();
        std::pin::Pin::new(&mut inner)
            .poll(cx)
            .map(|r| r.map(|_| self.clone()))
    }
}

#[doc(hidden)]
pub struct ResourceDataRef<'a, T>
where
    T: ResourceData + TypeUuidProvider,
{
    guard: MutexGuard<'a, ResourceState>,
    phantom: PhantomData<T>,
}

impl<'a, T> Debug for ResourceDataRef<'a, T>
where
    T: ResourceData + TypeUuidProvider,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self.guard {
            ResourceState::Pending { ref path, .. } => {
                write!(
                    f,
                    "Attempt to get reference to resource data while it is not loaded! Path is {}",
                    path.display()
                )
            }
            ResourceState::LoadError { ref path, .. } => {
                write!(
                    f,
                    "Attempt to get reference to resource data which failed to load! Path is {}",
                    path.display()
                )
            }
            ResourceState::Ok(ref data) => data.fmt(f),
        }
    }
}

impl<'a, T> Deref for ResourceDataRef<'a, T>
where
    T: ResourceData + TypeUuidProvider,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match *self.guard {
            ResourceState::Pending { ref path, .. } => {
                panic!(
                    "Attempt to get reference to resource data while it is not loaded! Path is {}",
                    path.display()
                )
            }
            ResourceState::LoadError { ref path, .. } => {
                panic!(
                    "Attempt to get reference to resource data which failed to load! Path is {}",
                    path.display()
                )
            }
            ResourceState::Ok(ref data) => {
                (**data).as_any().downcast_ref().expect("Type mismatch!")
            }
        }
    }
}

impl<'a, T> DerefMut for ResourceDataRef<'a, T>
where
    T: ResourceData + TypeUuidProvider,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match *self.guard {
            ResourceState::Pending { ref path, .. } => {
                panic!(
                    "Attempt to get reference to resource data while it is not loaded! Path is {}",
                    path.display()
                )
            }
            ResourceState::LoadError { ref path, .. } => {
                panic!(
                    "Attempt to get reference to resource data which failed to load! Path is {}",
                    path.display()
                )
            }
            ResourceState::Ok(ref mut data) => (**data)
                .as_any_mut()
                .downcast_mut()
                .expect("Type mismatch!"),
        }
    }
}
