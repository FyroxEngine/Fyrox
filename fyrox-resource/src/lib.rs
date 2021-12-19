//! Resource management

#![warn(missing_docs)]

use crate::core::parking_lot::{Mutex, MutexGuard};
use crate::core::visitor::prelude::*;
use std::{
    borrow::Cow,
    fmt::Debug,
    future::Future,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

pub use fyrox_core as core;

/// A trait for resource data.
pub trait ResourceData: 'static + Default + Debug + Visit + Send {
    /// Returns path of resource data.
    fn path(&self) -> Cow<Path>;

    /// Sets new path to resource data.
    fn set_path(&mut self, path: PathBuf);
}

/// A trait for resource load error.
pub trait ResourceLoadError: 'static + Debug + Send + Sync {}

impl<T> ResourceLoadError for T where T: 'static + Debug + Send + Sync {}

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
pub enum ResourceState<T: ResourceData, E: ResourceLoadError> {
    /// Resource is loading from external resource or in the queue to load.
    Pending {
        /// A path to load resource from.
        path: PathBuf,
        /// List of wakers to wake future when resource is fully loaded.
        wakers: Vec<Waker>,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: PathBuf,
        /// An error. This wrapped in Option only to be Default_ed.
        error: Option<Arc<E>>,
    },
    /// Actual resource data when it is fully loaded.
    Ok(T),
}

impl<T: ResourceData, E: ResourceLoadError> Visit for ResourceState<T, E> {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }

        match self {
            Self::Pending { path, .. } => panic!(
                "Resource {} must be .await_ed before serialization",
                path.display()
            ),
            // This may look strange if we attempting to save an invalid resource, but this may be
            // actually useful - a resource may become loadable at the deserialization.
            Self::LoadError { path, .. } => path.visit("Path", visitor)?,
            Self::Ok(details) => details.visit("Details", visitor)?,
        }

        visitor.leave_region()
    }
}

/// See module docs.
#[derive(Debug, Visit)]
pub struct Resource<T: ResourceData, E: ResourceLoadError> {
    state: Option<Arc<Mutex<ResourceState<T, E>>>>,
}

impl<T: ResourceData, E: ResourceLoadError> PartialEq for Resource<T, E> {
    fn eq(&self, other: &Self) -> bool {
        match (self.state.as_ref(), other.state.as_ref()) {
            (Some(state), Some(other_state)) => std::ptr::eq(&**state, &**other_state),
            (None, None) => true,
            _ => false,
        }
    }
}

#[doc(hidden)]
pub struct ResourceDataRef<'a, T: ResourceData, E: ResourceLoadError> {
    guard: MutexGuard<'a, ResourceState<T, E>>,
}

impl<'a, T: ResourceData, E: ResourceLoadError> Deref for ResourceDataRef<'a, T, E> {
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
            ResourceState::Ok(ref data) => data,
        }
    }
}

impl<'a, T: ResourceData, E: ResourceLoadError> DerefMut for ResourceDataRef<'a, T, E> {
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
            ResourceState::Ok(ref mut data) => data,
        }
    }
}

impl<T: ResourceData, E: ResourceLoadError> Resource<T, E> {
    /// Creates new resource with a given state.
    #[inline]
    pub fn new(state: ResourceState<T, E>) -> Self {
        Self {
            state: Some(Arc::new(Mutex::new(state))),
        }
    }

    /// Converts self to internal value.
    #[inline]
    pub fn into_inner(self) -> Arc<Mutex<ResourceState<T, E>>> {
        self.state.unwrap()
    }

    /// Locks internal mutex provides access to the state.
    #[inline]
    pub fn state(&self) -> MutexGuard<'_, ResourceState<T, E>> {
        self.state.as_ref().unwrap().lock()
    }

    /// Tries to lock internal mutex provides access to the state.
    #[inline]
    pub fn try_acquire_state(&self) -> Option<MutexGuard<'_, ResourceState<T, E>>> {
        self.state.as_ref().unwrap().try_lock()
    }

    /// Returns exact amount of users of the resource.
    #[inline]
    pub fn use_count(&self) -> usize {
        Arc::strong_count(self.state.as_ref().unwrap())
    }

    /// Returns a pointer as numeric value which can be used as a hash.
    #[inline]
    pub fn key(&self) -> usize {
        (&**self.state.as_ref().unwrap() as *const _) as usize
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
    pub fn data_ref(&self) -> ResourceDataRef<'_, T, E> {
        ResourceDataRef {
            guard: self.state(),
        }
    }
}

impl<T: ResourceData, E: ResourceLoadError> Default for Resource<T, E> {
    #[inline]
    fn default() -> Self {
        Self { state: None }
    }
}

impl<T: ResourceData, E: ResourceLoadError> Clone for Resource<T, E> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<T: ResourceData, E: ResourceLoadError> From<Arc<Mutex<ResourceState<T, E>>>>
    for Resource<T, E>
{
    #[inline]
    fn from(state: Arc<Mutex<ResourceState<T, E>>>) -> Self {
        Self { state: Some(state) }
    }
}

#[allow(clippy::from_over_into)]
impl<T: ResourceData, E: ResourceLoadError> Into<Arc<Mutex<ResourceState<T, E>>>>
    for Resource<T, E>
{
    #[inline]
    fn into(self) -> Arc<Mutex<ResourceState<T, E>>> {
        self.state.unwrap()
    }
}

impl<T: ResourceData, E: ResourceLoadError> Future for Resource<T, E> {
    type Output = Result<Self, Option<Arc<E>>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = self.as_ref().state.clone();
        match *state.unwrap().lock() {
            ResourceState::Pending { ref mut wakers, .. } => {
                // Collect wakers, so we'll be able to wake task when worker thread finish loading.
                let cx_waker = cx.waker();
                if let Some(pos) = wakers.iter().position(|waker| waker.will_wake(cx_waker)) {
                    wakers[pos] = cx_waker.clone();
                } else {
                    wakers.push(cx_waker.clone())
                }

                Poll::Pending
            }
            ResourceState::LoadError { ref error, .. } => Poll::Ready(Err(error.clone())),
            ResourceState::Ok(_) => Poll::Ready(Ok(self.clone())),
        }
    }
}

impl<T: ResourceData, E: ResourceLoadError> ResourceState<T, E> {
    /// Creates new resource in pending state.
    #[inline]
    pub fn new_pending(path: PathBuf) -> Self {
        Self::Pending {
            path,
            wakers: Default::default(),
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

    #[inline]
    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Pending {
                path: Default::default(),
                wakers: Default::default(),
            }),
            1 => Ok(Self::LoadError {
                path: Default::default(),
                error: None,
            }),
            2 => Ok(Self::Ok(Default::default())),
            _ => Err(format!("Invalid resource id {}", id)),
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
    pub fn commit(&mut self, state: ResourceState<T, E>) {
        let wakers = if let ResourceState::Pending { ref mut wakers, .. } = self {
            std::mem::take(wakers)
        } else {
            unreachable!()
        };

        *self = state;

        for waker in wakers {
            waker.wake();
        }
    }
}

impl<T: ResourceData, E: ResourceLoadError> Default for ResourceState<T, E> {
    fn default() -> Self {
        Self::Ok(Default::default())
    }
}

/// Defines a new resource type via new-type wrapper.
#[macro_export]
macro_rules! define_new_resource {
    ($(#[$meta:meta])* $name:ident<$state:ty, $error:ty>) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Default, PartialEq)]
        #[repr(transparent)]
        pub struct $name(pub Resource<$state, $error>);

        impl Visit for $name {
            fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
                self.0.visit(name, visitor)
            }
        }

        impl std::ops::Deref for $name {
            type Target = Resource<$state, $error>;

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
            type Output = Result<Self, Option<std::sync::Arc<$error>>>;

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
