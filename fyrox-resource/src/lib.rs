//! Resource management

#![warn(missing_docs)]

use crate::{
    constructor::ResourceConstructorContainer,
    core::{
        curve::Curve,
        parking_lot::{Mutex, MutexGuard},
        uuid::{uuid, Uuid},
        visitor::{prelude::*, RegionGuard},
    },
};
use std::{
    any::Any,
    borrow::Cow,
    fmt::{Debug, Formatter},
    future::Future,
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

pub mod constructor;

pub use fyrox_core as core;

pub const LEGACY_TEXTURE_RESOURCE_UUID: Uuid = uuid!("02c23a44-55fa-411a-bc39-eb7a5eadf15c");
pub const LEGACY_SOUND_BUFFER_RESOURCE_UUID: Uuid = uuid!("f6a077b7-c8ff-4473-a95b-0289441ea9d8");
pub const LEGACY_SHADER_RESOURCE_UUID: Uuid = uuid!("f1346417-b726-492a-b80f-c02096c6c019");
pub const LEGACY_CURVE_RESOURCE_UUID: Uuid = uuid!("f28b949f-28a2-4b68-9089-59c234f58b6b");

fn guess_uuid(region: &mut RegionGuard) -> Uuid {
    assert!(region.is_reading());

    let mut mip_count = 0;
    if mip_count.visit("MipCount", region).is_ok() {
        return LEGACY_TEXTURE_RESOURCE_UUID;
    }

    let mut curve = Curve::default();
    if curve.visit("Curve", region).is_ok() {
        return LEGACY_CURVE_RESOURCE_UUID;
    }

    let mut id = 0u32;
    if id.visit("Id", region).is_ok() {
        return LEGACY_SOUND_BUFFER_RESOURCE_UUID;
    }

    // This is unreliable, but shader does not contain anything special that could be used
    // for identification.
    LEGACY_SHADER_RESOURCE_UUID
}

/// A trait for resource data.
pub trait ResourceData: 'static + Debug + Visit + Send {
    /// Returns path of resource data.
    fn path(&self) -> Cow<Path>;

    /// Sets new path to resource data.
    fn set_path(&mut self, path: PathBuf);

    fn as_any(&self) -> &dyn Any;

    fn as_mut_mut(&mut self) -> &mut dyn Any;

    fn type_uuid(&self) -> Uuid;
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
pub enum ResourceState {
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
        error: Option<Arc<dyn ResourceLoadError>>,
    },
    /// Actual resource data when it is fully loaded.
    Ok(Box<dyn ResourceData>),
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

                    *self = Self::Pending {
                        path,
                        wakers: Default::default(),
                    };

                    Ok(())
                } else if let Self::Pending { path, .. } = self {
                    path.visit("Path", &mut region)
                } else {
                    Err(VisitError::User("Enum variant mismatch!".to_string()))
                }
            }
            1 => {
                if region.is_reading() {
                    let mut path = PathBuf::new();
                    path.visit("Path", &mut region)?;

                    *self = Self::LoadError { path, error: None };

                    Ok(())
                } else if let Self::LoadError { path, .. } = self {
                    path.visit("Path", &mut region)
                } else {
                    Err(VisitError::User("Enum variant mismatch!".to_string()))
                }
            }
            2 => {
                if region.is_reading() {
                    let mut type_uuid = Uuid::default();
                    if let Err(_) = type_uuid.visit("TypeUuid", &mut region) {
                        // We might be reading the old version, try to guess an actual type uuid by
                        // the inner content of the resource data.
                        type_uuid = guess_uuid(&mut region);
                    }

                    let constructors_container = region
                        .blackboard
                        .get::<ResourceConstructorContainer>()
                        .expect(
                            "Resource data constructor container must be \
                provided when serializing resources!",
                        );

                    if let Some(mut instance) = constructors_container.try_create(&type_uuid) {
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

/// See module docs.
#[derive(Visit)]
pub struct Resource {
    state: Option<Arc<Mutex<ResourceState>>>,
}

impl Debug for Resource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Resource")
    }
}

impl PartialEq for Resource {
    fn eq(&self, other: &Self) -> bool {
        match (self.state.as_ref(), other.state.as_ref()) {
            (Some(state), Some(other_state)) => std::ptr::eq(&**state, &**other_state),
            (None, None) => true,
            _ => false,
        }
    }
}

impl Eq for Resource {}

impl Hash for Resource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.state.as_ref() {
            None => state.write_u64(0),
            Some(resource_state) => state.write_u64(&**resource_state as *const _ as u64),
        }
    }
}

#[doc(hidden)]
pub struct ResourceDataRef<'a> {
    guard: MutexGuard<'a, ResourceState>,
}

impl<'a> Debug for ResourceDataRef<'a> {
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

impl<'a> Deref for ResourceDataRef<'a> {
    type Target = dyn ResourceData;

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
            ResourceState::Ok(ref data) => &**data,
        }
    }
}

impl<'a> DerefMut for ResourceDataRef<'a> {
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
            ResourceState::Ok(ref mut data) => &mut **data,
        }
    }
}

impl Resource {
    /// Creates new resource with a given state.
    #[inline]
    pub fn new(state: ResourceState) -> Self {
        Self {
            state: Some(Arc::new(Mutex::new(state))),
        }
    }

    /// Converts self to internal value.
    #[inline]
    pub fn into_inner(self) -> Arc<Mutex<ResourceState>> {
        self.state.unwrap()
    }

    /// Locks internal mutex provides access to the state.
    #[inline]
    pub fn state(&self) -> MutexGuard<'_, ResourceState> {
        self.state.as_ref().unwrap().lock()
    }

    /// Returns true if the resource is still loading.
    pub fn is_loading(&self) -> bool {
        matches!(*self.state(), ResourceState::Pending { .. })
    }

    /// Tries to lock internal mutex provides access to the state.
    #[inline]
    pub fn try_acquire_state(&self) -> Option<MutexGuard<'_, ResourceState>> {
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
    pub fn data_ref(&self) -> ResourceDataRef<'_> {
        ResourceDataRef {
            guard: self.state(),
        }
    }
}

impl Default for Resource {
    #[inline]
    fn default() -> Self {
        Self { state: None }
    }
}

impl Clone for Resource {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl From<Arc<Mutex<ResourceState>>> for Resource {
    #[inline]
    fn from(state: Arc<Mutex<ResourceState>>) -> Self {
        Self { state: Some(state) }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Arc<Mutex<ResourceState>>> for Resource {
    #[inline]
    fn into(self) -> Arc<Mutex<ResourceState>> {
        self.state.unwrap()
    }
}

impl Future for Resource {
    type Output = Result<Self, Option<Arc<dyn ResourceLoadError>>>;

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

impl ResourceState {
    /// Creates new resource in pending state.
    #[inline]
    pub fn new_pending(path: PathBuf) -> Self {
        Self::Pending {
            path,
            wakers: Default::default(),
        }
    }

    /// Switches the internal state of the resource to [`ResourceState::Pending`].
    pub fn switch_to_pending_state(&mut self) {
        match self {
            ResourceState::LoadError { path, .. } => {
                *self = ResourceState::Pending {
                    path: std::mem::take(path),
                    wakers: Default::default(),
                }
            }
            ResourceState::Ok(data) => {
                *self = ResourceState::Pending {
                    path: data.path().to_path_buf(),
                    wakers: Default::default(),
                }
            }
            _ => (),
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
    pub fn commit(&mut self, state: ResourceState) {
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

    /// Changes internal state to [`ResourceState::Ok`]
    pub fn commit_ok<T: ResourceData>(&mut self, data: T) {
        self.commit(ResourceState::Ok(Box::new(data)))
    }

    /// Changes internal state to [`ResourceState::LoadError`].
    pub fn commit_error<E: ResourceLoadError>(&mut self, path: PathBuf, error: E) {
        self.commit(ResourceState::LoadError {
            path,
            error: Some(Arc::new(error)),
        })
    }
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::LoadError {
            error: None,
            path: Default::default(),
        }
    }
}

/// Defines a new resource type via new-type wrapper.
#[macro_export]
macro_rules! define_new_resource {
    ($(#[$meta:meta])* $name:ident<$state:ty, $error:ty>) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        pub struct $name(pub Resource<$state, $error>);

        impl Visit for $name {
            fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
                self.0.visit(name, visitor)
            }
        }

        impl From<Resource<$state, $error>> for $name {
            fn from(resource: Resource<$state, $error>) -> Self {
                $name(resource)
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
