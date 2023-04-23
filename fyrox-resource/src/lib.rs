//! Resource management

#![warn(missing_docs)]

use crate::{
    core::{
        parking_lot::MutexGuard,
        reflect::FieldValue,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    state::ResourceState,
};
use std::fmt::Display;
use std::{
    any::Any,
    borrow::Cow,
    fmt::{Debug, Formatter},
    future::Future,
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::untyped::UntypedResource;
pub use fyrox_core as core;

pub mod constructor;
pub mod container;
pub mod loader;
pub mod manager;
pub mod options;
pub mod state;
pub mod task;
pub mod untyped;

pub const TEXTURE_RESOURCE_UUID: Uuid = uuid!("02c23a44-55fa-411a-bc39-eb7a5eadf15c");
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
pub trait ResourceLoadError: 'static + Debug + Display + Send + Sync {}

impl<T> ResourceLoadError for T where T: 'static + Display + Debug + Send + Sync {}

pub struct ResourceStateGuard<'a, T>
where
    T: ResourceData,
{
    guard: MutexGuard<'a, ResourceState>,
    phantom: PhantomData<T>,
}

impl<'a, T> ResourceStateGuard<'a, T>
where
    T: ResourceData,
{
    pub fn get(&mut self) -> ResourceStateRef<'_, T> {
        match &*self.guard {
            ResourceState::Pending { path, .. } => ResourceStateRef::Pending { path },
            ResourceState::LoadError { path, error } => ResourceStateRef::LoadError { path, error },
            ResourceState::Ok(data) => {
                ResourceStateRef::Ok(data.as_any().downcast_ref().expect("Type mismatch!"))
            }
        }
    }

    pub fn get_mut(&mut self) -> ResourceStateRefMut<'_, T> {
        match &mut *self.guard {
            ResourceState::Pending { path, .. } => ResourceStateRefMut::Pending { path },
            ResourceState::LoadError { path, error } => {
                ResourceStateRefMut::LoadError { path, error }
            }
            ResourceState::Ok(data) => {
                ResourceStateRefMut::Ok(data.as_any_mut().downcast_mut().expect("Type mismatch!"))
            }
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
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: &'a PathBuf,
        /// An error.
        error: &'a Option<Arc<dyn ResourceLoadError>>,
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
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: &'a mut PathBuf,
        /// An error.
        error: &'a mut Option<Arc<dyn ResourceLoadError>>,
    },
    /// Actual resource data when it is fully loaded.
    Ok(&'a mut T),
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::LoadError {
            error: None,
            path: Default::default(),
        }
    }
}

#[derive(Visit, PartialEq, Eq, Hash, Debug)]
pub struct Resource<T>
where
    T: ResourceData,
{
    state: Option<UntypedResource>,
    #[visit(skip)]
    phantom: PhantomData<T>,
}

impl<T> Resource<T>
where
    T: ResourceData,
{
    pub fn new_pending(path: PathBuf) -> Self {
        Self {
            state: Some(UntypedResource::new_pending(path)),
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
            state: Some(UntypedResource::new_load_error(path, error)),
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
    T: ResourceData,
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
    T: ResourceData,
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
    T: ResourceData,
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
    T: ResourceData,
{
    #[inline]
    fn into(self) -> UntypedResource {
        self.state.unwrap()
    }
}

impl<T> Future for Resource<T>
where
    T: ResourceData,
{
    type Output = Result<Self, Option<Arc<dyn ResourceLoadError>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.state.as_ref().unwrap().clone();
        std::pin::Pin::new(&mut inner)
            .poll(cx)
            .map(|r| r.map(|_| self.clone()))
    }
}

#[doc(hidden)]
pub struct ResourceDataRef<'a, T>
where
    T: ResourceData,
{
    guard: MutexGuard<'a, ResourceState>,
    phantom: PhantomData<T>,
}

impl<'a, T> Debug for ResourceDataRef<'a, T>
where
    T: ResourceData,
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
    T: ResourceData,
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
            ResourceState::Ok(ref data) => data.as_any().downcast_ref().expect("Type mismatch!"),
        }
    }
}

impl<'a, T> DerefMut for ResourceDataRef<'a, T>
where
    T: ResourceData,
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
            ResourceState::Ok(ref mut data) => {
                data.as_any_mut().downcast_mut().expect("Type mismatch!")
            }
        }
    }
}

/// Defines a new resource type via new-type wrapper.
#[macro_export]
macro_rules! define_new_resource {
    ($(#[$meta:meta])* $name:ident<$state:ty>) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        pub struct $name(pub Resource<$state>);

        impl Visit for $name {
            fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
                self.0.visit(name, visitor)
            }
        }

        impl From<Resource<$state>> for $name {
            fn from(resource: Resource<$state>) -> Self {
                $name(resource)
            }
        }

        impl std::ops::Deref for $name {
            type Target = Resource<$state>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl std::future::Future for $name {
            type Output = Result<Self, Option<std::sync::Arc<dyn $crate::ResourceLoadError>>>;

            fn poll(
                mut self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                std::pin::Pin::new(&mut self.0)
                    .poll(cx)
                    .map(|r| r.map(|_| self.clone()))
            }
        }
    };
}
