//! Resource management

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use crate::{
    core::{
        parking_lot::MutexGuard,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
        TypeUuidProvider,
    },
    manager::ResourceManager,
    state::ResourceState,
    untyped::UntypedResource,
};
use fxhash::FxHashSet;
use std::error::Error;
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

pub use fyrox_core as core;
use fyrox_core::log::Log;

pub mod constructor;
pub mod entry;
pub mod event;
pub mod graph;
pub mod io;
pub mod loader;
pub mod manager;
pub mod options;
pub mod state;
mod task;
pub mod untyped;

/// Type UUID of texture resource. It is defined here to load old versions of resources.
pub const TEXTURE_RESOURCE_UUID: Uuid = uuid!("02c23a44-55fa-411a-bc39-eb7a5eadf15c");
/// Type UUID of model resource. It is defined here to load old versions of resources.
pub const MODEL_RESOURCE_UUID: Uuid = uuid!("44cd768f-b4ca-4804-a98c-0adf85577ada");
/// Type UUID of sound buffer resource. It is defined here to load old versions of resources.
pub const SOUND_BUFFER_RESOURCE_UUID: Uuid = uuid!("f6a077b7-c8ff-4473-a95b-0289441ea9d8");
/// Type UUID of shader resource. It is defined here to load old versions of resources.
pub const SHADER_RESOURCE_UUID: Uuid = uuid!("f1346417-b726-492a-b80f-c02096c6c019");
/// Type UUID of curve resource. It is defined here to load old versions of resources.
pub const CURVE_RESOURCE_UUID: Uuid = uuid!("f28b949f-28a2-4b68-9089-59c234f58b6b");

/// A trait for resource data.
pub trait ResourceData: 'static + Debug + Visit + Send + Reflect {
    /// Returns path of resource data.
    fn path(&self) -> Cow<Path>;

    /// Sets new path to resource data.
    fn set_path(&mut self, path: PathBuf);

    /// Returns `self` as `&dyn Any`. It is useful to implement downcasting to a particular type.
    fn as_any(&self) -> &dyn Any;

    /// Returns `self` as `&mut dyn Any`. It is useful to implement downcasting to a particular type.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Returns unique data type id.
    fn type_uuid(&self) -> Uuid;

    /// Returns true if the resource data was generated procedurally, not taken from a file.
    fn is_procedural(&self) -> bool;

    /// Saves the resource data a file at the specified path. By default, this method returns an
    /// error that tells that saving functionality is not implemented. This method is free to
    /// decide how the resource data is saved. This is needed, because there are multiple formats
    /// that defines various kinds of resources. For example, a rectangular texture could be saved
    /// into a whole bunch of formats, such as png, bmp, tga, jpg etc, but in the engine it is single
    /// Texture resource. In any case, produced file should be compatible with a respective resource
    /// loader.
    fn save(&mut self, #[allow(unused_variables)] path: &Path) -> Result<(), Box<dyn Error>> {
        Err("Saving is not supported!".to_string().into())
    }
}

/// A trait for resource load error.
pub trait ResourceLoadError: 'static + Debug + Send + Sync {}

impl<T> ResourceLoadError for T where T: 'static + Debug + Send + Sync {}

/// Provides typed access to a resource state.
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
    /// Fetches the actual state of the resource.
    pub fn get(&self) -> ResourceStateRef<'_, T> {
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
                ResourceData::as_any(&**data)
                    .downcast_ref()
                    .expect("Type mismatch!"),
            ),
        }
    }

    /// Fetches the actual state of the resource.
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
                ResourceData::as_any_mut(&mut **data)
                    .downcast_mut()
                    .expect("Type mismatch!"),
            ),
        }
    }
}

/// Provides typed access to a resource state.
#[derive(Debug)]
pub enum ResourceStateRef<'a, T>
where
    T: ResourceData,
{
    /// Resource is loading from external resource or in the queue to load.
    Pending {
        /// A path to load resource from.
        path: &'a PathBuf,
        /// Actual resource type id.
        type_uuid: Uuid,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: &'a PathBuf,
        /// An error.
        error: &'a Option<Arc<dyn ResourceLoadError>>,
        /// Actual resource type id.
        type_uuid: Uuid,
    },
    /// Actual resource data when it is fully loaded.
    Ok(&'a T),
}

/// Provides typed access to a resource state.
#[derive(Debug)]
pub enum ResourceStateRefMut<'a, T> {
    /// Resource is loading from external resource or in the queue to load.
    Pending {
        /// A path to load resource from.
        path: &'a mut PathBuf,
        /// Actual resource type id.
        type_uuid: Uuid,
    },
    /// An error has occurred during the load.
    LoadError {
        /// A path at which it was impossible to load the resource.
        path: &'a mut PathBuf,
        /// An error.
        error: &'a mut Option<Arc<dyn ResourceLoadError>>,
        /// Actual resource type id.
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

/// A resource of particular data type.
#[derive(Debug, Reflect)]
pub struct Resource<T>
where
    T: ResourceData + TypeUuidProvider,
{
    state: Option<UntypedResource>,
    #[reflect(hidden)]
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

            // There might be a built-in resource, in this case we must restore the "reference" to it.
            let state = resource_manager.state();
            if let Some(built_in_resource) = state.built_in_resources.get(&path) {
                if built_in_resource.type_uuid() == self.state.as_ref().unwrap().type_uuid() {
                    self.state = Some(built_in_resource.clone());
                } else {
                    Log::err(format!(
                        "Built in resource {:?} has changed its type and cannot be restored!",
                        path
                    ));
                }
            } else {
                drop(state);
                let is_procedural = self.state.as_ref().unwrap().is_procedural();
                if !is_procedural {
                    self.state = Some(resource_manager.request_untyped(path));
                }
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
    /// Creates new resource in pending state.
    #[inline]
    pub fn new_pending(path: PathBuf) -> Self {
        Self {
            state: Some(UntypedResource::new_pending(
                path,
                <T as TypeUuidProvider>::type_uuid(),
            )),
            phantom: PhantomData,
        }
    }

    /// Creates new resource in ok state (fully loaded).
    #[inline]
    pub fn new_ok(data: T) -> Self {
        Self {
            state: Some(UntypedResource::new_ok(data)),
            phantom: PhantomData,
        }
    }

    /// Creates new resource in error state.
    #[inline]
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
    pub fn into_untyped(self) -> UntypedResource {
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
        self.state
            .as_ref()
            .unwrap()
            .0
            .try_lock()
            .map(|guard| ResourceStateGuard {
                guard,
                phantom: Default::default(),
            })
    }

    fn state_inner(&self) -> MutexGuard<'_, ResourceState> {
        self.state.as_ref().unwrap().0.lock()
    }

    /// Returns true if the resource is still loading.
    #[inline]
    pub fn is_loading(&self) -> bool {
        matches!(*self.state_inner(), ResourceState::Pending { .. })
    }

    /// Returns true if the resource is fully loaded and ready for use.
    #[inline]
    pub fn is_ok(&self) -> bool {
        matches!(*self.state_inner(), ResourceState::Ok(_))
    }

    /// Returns true if the resource is failed to load.
    #[inline]
    pub fn is_failed_to_load(&self) -> bool {
        matches!(*self.state_inner(), ResourceState::LoadError { .. })
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

    /// Returns path of the resource.
    #[inline]
    pub fn path(&self) -> PathBuf {
        self.state.as_ref().unwrap().0.lock().path().to_path_buf()
    }

    /// Sets a new path of the resource.
    #[inline]
    pub fn set_path(&mut self, new_path: PathBuf) {
        self.state.as_ref().unwrap().set_path(new_path);
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
        Pin::new(&mut inner)
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
            ResourceState::Ok(ref data) => ResourceData::as_any(&**data)
                .downcast_ref()
                .expect("Type mismatch!"),
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
            ResourceState::Ok(ref mut data) => ResourceData::as_any_mut(&mut **data)
                .downcast_mut()
                .expect("Type mismatch!"),
        }
    }
}

/// Collects all resources used by a given entity. Internally, it uses reflection to iterate over
/// each field of every descendant sub-object of the entity. This function could be used to collect
/// all resources used by an object, which could be useful if you're building a resource dependency
/// analyzer.
pub fn collect_used_resources(
    entity: &dyn Reflect,
    resources_collection: &mut FxHashSet<UntypedResource>,
) {
    let mut finished = false;

    entity.downcast_ref::<UntypedResource>(&mut |v| {
        if let Some(resource) = v {
            resources_collection.insert(resource.clone());
            finished = true;
        }
    });

    if finished {
        return;
    }

    entity.as_array(&mut |array| {
        if let Some(array) = array {
            for i in 0..array.reflect_len() {
                if let Some(item) = array.reflect_index(i) {
                    collect_used_resources(item, resources_collection)
                }
            }

            finished = true;
        }
    });

    if finished {
        return;
    }

    entity.as_inheritable_variable(&mut |inheritable| {
        if let Some(inheritable) = inheritable {
            collect_used_resources(inheritable.inner_value_ref(), resources_collection);

            finished = true;
        }
    });

    if finished {
        return;
    }

    entity.as_hash_map(&mut |hash_map| {
        if let Some(hash_map) = hash_map {
            for i in 0..hash_map.reflect_len() {
                if let Some((key, value)) = hash_map.reflect_get_at(i) {
                    collect_used_resources(key, resources_collection);
                    collect_used_resources(value, resources_collection);
                }
            }

            finished = true;
        }
    });

    if finished {
        return;
    }

    entity.fields(&mut |fields| {
        for field in fields {
            collect_used_resources(field, resources_collection);
        }
    })
}
