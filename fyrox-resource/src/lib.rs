//! Resource management

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use crate::{
    core::{
        parking_lot::MutexGuard,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
        TypeUuidProvider,
    },
    state::ResourceState,
    untyped::UntypedResource,
};
use fxhash::FxHashSet;
use std::{
    any::Any,
    error::Error,
    fmt::{Debug, Formatter},
    future::Future,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::Path,
    pin::Pin,
    task::{Context, Poll},
};

use crate::state::LoadError;
use crate::untyped::{ResourceHeader, ResourceKind};
pub use fyrox_core as core;
use fyrox_core::combine_uuids;

pub mod constructor;
pub mod entry;
pub mod event;
pub mod graph;
pub mod io;
pub mod loader;
pub mod manager;
pub mod options;
pub mod state;
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
    /// Returns `self` as `&dyn Any`. It is useful to implement downcasting to a particular type.
    fn as_any(&self) -> &dyn Any;

    /// Returns `self` as `&mut dyn Any`. It is useful to implement downcasting to a particular type.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Returns unique data type id.
    fn type_uuid(&self) -> Uuid;

    /// Saves the resource data a file at the specified path. This method is free to
    /// decide how the resource data is saved. This is needed, because there are multiple formats
    /// that defines various kinds of resources. For example, a rectangular texture could be saved
    /// into a whole bunch of formats, such as png, bmp, tga, jpg etc, but in the engine it is single
    /// Texture resource. In any case, produced file should be compatible with a respective resource
    /// loader.
    fn save(&mut self, #[allow(unused_variables)] path: &Path) -> Result<(), Box<dyn Error>>;

    /// Returns `true` if the resource data can be saved to a file, `false` - otherwise. Not every
    /// resource type supports saving, for example there might be temporary resource type that is
    /// used only at runtime which does not need saving at all.
    fn can_be_saved(&self) -> bool;
}

/// Extension trait for a resource data of a particular type, which adds additional functionality,
/// such as: a way to get default state of the data (`Default` impl), a way to get data's type uuid.
/// The trait has automatic implementation for any type that implements
/// ` ResourceData + Default + TypeUuidProvider` traits.
pub trait TypedResourceData: ResourceData + Default + TypeUuidProvider {}

impl<T> TypedResourceData for T where T: ResourceData + Default + TypeUuidProvider {}

/// A trait for resource load error.
pub trait ResourceLoadError: 'static + Debug + Send + Sync {}

impl<T> ResourceLoadError for T where T: 'static + Debug + Send + Sync {}

/// Provides typed access to a resource state.
pub struct ResourceHeaderGuard<'a, T>
where
    T: TypedResourceData,
{
    guard: MutexGuard<'a, ResourceHeader>,
    phantom: PhantomData<T>,
}

impl<'a, T> ResourceHeaderGuard<'a, T>
where
    T: TypedResourceData,
{
    pub fn kind(&self) -> &ResourceKind {
        &self.guard.kind
    }

    pub fn data(&mut self) -> Option<&mut T> {
        if let ResourceState::Ok(ref mut data) = self.guard.state {
            ResourceData::as_any_mut(&mut **data).downcast_mut::<T>()
        } else {
            None
        }
    }
}

/// A resource of particular data type. It is a typed wrapper around [`UntypedResource`] which
/// does type checks at runtime.
///
/// ## Default State
///
/// Default state of the resource will be [`ResourceState::Ok`] with `T::default`.
#[derive(Debug, Reflect)]
pub struct Resource<T>
where
    T: TypedResourceData,
{
    untyped: UntypedResource,
    #[reflect(hidden)]
    phantom: PhantomData<T>,
}

impl<T: TypedResourceData> TypeUuidProvider for Resource<T> {
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid!("790b1a1c-a997-46c4-ac3b-8565501f0052"),
            <T as TypeUuidProvider>::type_uuid(),
        )
    }
}

impl<T> Visit for Resource<T>
where
    T: TypedResourceData,
{
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        // Backward compatibility.
        if region.is_reading() {
            let mut old_option_wrapper: Option<UntypedResource> = None;
            if old_option_wrapper.visit("State", &mut region).is_ok() {
                self.untyped = old_option_wrapper.unwrap();
            } else {
                self.untyped.visit("State", &mut region)?;
            }
        } else {
            self.untyped.visit("State", &mut region)?;
        }

        Ok(())
    }
}

impl<T> PartialEq for Resource<T>
where
    T: TypedResourceData,
{
    fn eq(&self, other: &Self) -> bool {
        self.untyped == other.untyped
    }
}

impl<T> Eq for Resource<T> where T: TypedResourceData {}

impl<T> Hash for Resource<T>
where
    T: TypedResourceData,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.untyped.hash(state)
    }
}

impl<T> Resource<T>
where
    T: TypedResourceData,
{
    /// Creates new resource in pending state.
    #[inline]
    pub fn new_pending(kind: ResourceKind) -> Self {
        Self {
            untyped: UntypedResource::new_pending(kind, <T as TypeUuidProvider>::type_uuid()),
            phantom: PhantomData,
        }
    }

    /// Creates new resource in ok state (fully loaded).
    #[inline]
    pub fn new_ok(kind: ResourceKind, data: T) -> Self {
        Self {
            untyped: UntypedResource::new_ok(kind, data),
            phantom: PhantomData,
        }
    }

    /// Creates new resource in error state.
    #[inline]
    pub fn new_load_error(kind: ResourceKind, error: LoadError) -> Self {
        Self {
            untyped: UntypedResource::new_load_error(
                kind,
                error,
                <T as TypeUuidProvider>::type_uuid(),
            ),
            phantom: PhantomData,
        }
    }

    /// Converts self to internal value.
    #[inline]
    pub fn into_untyped(self) -> UntypedResource {
        self.untyped
    }

    /// Locks internal mutex provides access to the state.
    #[inline]
    pub fn state(&self) -> ResourceHeaderGuard<'_, T> {
        let guard = self.untyped.0.lock();
        ResourceHeaderGuard {
            guard,
            phantom: Default::default(),
        }
    }

    /// Tries to lock internal mutex provides access to the state.
    #[inline]
    pub fn try_acquire_state(&self) -> Option<ResourceHeaderGuard<'_, T>> {
        self.untyped.0.try_lock().map(|guard| ResourceHeaderGuard {
            guard,
            phantom: Default::default(),
        })
    }

    #[inline]
    pub fn header(&self) -> MutexGuard<'_, ResourceHeader> {
        self.untyped.0.lock()
    }

    /// Returns true if the resource is still loading.
    #[inline]
    pub fn is_loading(&self) -> bool {
        matches!(self.untyped.0.lock().state, ResourceState::Pending { .. })
    }

    /// Returns true if the resource is fully loaded and ready for use.
    #[inline]
    pub fn is_ok(&self) -> bool {
        matches!(self.untyped.0.lock().state, ResourceState::Ok(_))
    }

    /// Returns true if the resource is failed to load.
    #[inline]
    pub fn is_failed_to_load(&self) -> bool {
        matches!(self.untyped.0.lock().state, ResourceState::LoadError { .. })
    }

    /// Returns exact amount of users of the resource.
    #[inline]
    pub fn use_count(&self) -> usize {
        self.untyped.use_count()
    }

    /// Returns a pointer as numeric value which can be used as a hash.
    #[inline]
    pub fn key(&self) -> u64 {
        self.untyped.key() as u64
    }

    /// Returns kind of the resource.
    #[inline]
    pub fn kind(&self) -> ResourceKind {
        self.untyped.kind()
    }

    /// Sets a new kind of the resource.
    #[inline]
    pub fn set_path(&mut self, new_kind: ResourceKind) {
        self.untyped.set_kind(new_kind);
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
            guard: self.untyped.0.lock(),
            phantom: Default::default(),
        }
    }

    /// Tries to save the resource to the specified path.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.untyped.save(path)
    }

    /// Tries to save the resource back to its external location. This method will fail on attempt
    /// to save embedded resource, because embedded resources does not have external location.
    pub fn save_back(&self) -> Result<(), Box<dyn Error>> {
        self.untyped.save_back()
    }
}

impl<T> Default for Resource<T>
where
    T: TypedResourceData,
{
    #[inline]
    fn default() -> Self {
        Self {
            untyped: UntypedResource::new_ok(Default::default(), T::default()),
            phantom: Default::default(),
        }
    }
}

impl<T> Clone for Resource<T>
where
    T: TypedResourceData,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            untyped: self.untyped.clone(),
            phantom: Default::default(),
        }
    }
}

impl<T> From<UntypedResource> for Resource<T>
where
    T: TypedResourceData,
{
    #[inline]
    fn from(untyped: UntypedResource) -> Self {
        assert_eq!(untyped.type_uuid(), <T as TypeUuidProvider>::type_uuid());
        Self {
            untyped,
            phantom: Default::default(),
        }
    }
}

#[allow(clippy::from_over_into)]
impl<T> Into<UntypedResource> for Resource<T>
where
    T: TypedResourceData,
{
    #[inline]
    fn into(self) -> UntypedResource {
        self.untyped
    }
}

impl<T> Future for Resource<T>
where
    T: TypedResourceData,
{
    type Output = Result<Self, LoadError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.untyped.clone();
        Pin::new(&mut inner)
            .poll(cx)
            .map(|r| r.map(|_| self.clone()))
    }
}

#[doc(hidden)]
pub struct ResourceDataRef<'a, T>
where
    T: TypedResourceData,
{
    guard: MutexGuard<'a, ResourceHeader>,
    phantom: PhantomData<T>,
}

impl<'a, T> ResourceDataRef<'a, T>
where
    T: TypedResourceData,
{
    #[inline]
    pub fn as_loaded_ref(&self) -> Option<&T> {
        match self.guard.state {
            ResourceState::Ok(ref data) => ResourceData::as_any(&**data).downcast_ref(),
            _ => None,
        }
    }

    #[inline]
    pub fn as_loaded_mut(&mut self) -> Option<&mut T> {
        match self.guard.state {
            ResourceState::Ok(ref mut data) => ResourceData::as_any_mut(&mut **data).downcast_mut(),
            _ => None,
        }
    }
}

impl<'a, T> Debug for ResourceDataRef<'a, T>
where
    T: TypedResourceData,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.guard.state {
            ResourceState::Pending { .. } => {
                write!(
                    f,
                    "Attempt to get reference to resource data while it is not loaded! Path is {}",
                    self.guard.kind
                )
            }
            ResourceState::LoadError { .. } => {
                write!(
                    f,
                    "Attempt to get reference to resource data which failed to load! Path is {}",
                    self.guard.kind
                )
            }
            ResourceState::Ok(ref data) => data.fmt(f),
        }
    }
}

impl<'a, T> Deref for ResourceDataRef<'a, T>
where
    T: TypedResourceData,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self.guard.state {
            ResourceState::Pending { .. } => {
                panic!(
                    "Attempt to get reference to resource data while it is not loaded! Path is {}",
                    self.guard.kind
                )
            }
            ResourceState::LoadError { .. } => {
                panic!(
                    "Attempt to get reference to resource data which failed to load! Path is {}",
                    self.guard.kind
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
    T: TypedResourceData,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        let header = &mut *self.guard;
        match header.state {
            ResourceState::Pending { .. } => {
                panic!(
                    "Attempt to get reference to resource data while it is not loaded! Path is {}",
                    header.kind
                )
            }
            ResourceState::LoadError { .. } => {
                panic!(
                    "Attempt to get reference to resource data which failed to load! Path is {}",
                    header.kind
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
    #[inline(always)]
    fn type_is<T: Reflect>(entity: &dyn Reflect) -> bool {
        let mut types_match = false;
        entity.downcast_ref::<T>(&mut |v| {
            types_match = v.is_some();
        });
        types_match
    }

    // Skip potentially large chunks of numeric data, that definitely cannot contain any resources.
    // TODO: This is a brute-force solution which does not include all potential types with plain
    // data.
    let mut finished = type_is::<Vec<u8>>(entity)
        || type_is::<Vec<u16>>(entity)
        || type_is::<Vec<u32>>(entity)
        || type_is::<Vec<u64>>(entity)
        || type_is::<Vec<i8>>(entity)
        || type_is::<Vec<i16>>(entity)
        || type_is::<Vec<i32>>(entity)
        || type_is::<Vec<i64>>(entity)
        || type_is::<Vec<f32>>(entity)
        || type_is::<Vec<f64>>(entity);

    if finished {
        return;
    }

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
            collect_used_resources(*field, resources_collection);
        }
    })
}
