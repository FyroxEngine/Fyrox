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

//! Resource management

#![forbid(unsafe_code)]
#![allow(clippy::doc_lazy_continuation)]
#![allow(clippy::mutable_key_type)]
#![warn(missing_docs)]

use crate::{
    core::{
        combine_uuids,
        parking_lot::MutexGuard,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
        TypeUuidProvider,
    },
    state::{LoadError, ResourceState},
    untyped::{ResourceHeader, ResourceKind, UntypedResource},
};
use fxhash::FxHashSet;
pub use fyrox_core as core;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    error::Error,
    fmt::Display,
    fmt::{Debug, Formatter},
    future::Future,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::Path,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

pub mod builtin;
pub mod constructor;
pub mod entry;
pub mod event;
pub mod graph;
pub mod io;
pub mod loader;
pub mod manager;
pub mod metadata;
pub mod options;
pub mod registry;
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
/// Type UUID of HRIR sphere resource. It is defined here to load old versions of resources.
pub const HRIR_SPHERE_RESOURCE_UUID: Uuid = uuid!("c92a0fa3-0ed3-49a9-be44-8f06271c6be2");
/// Type UUID of font resource. It is defined here to load old versions of resources.
pub const FONT_RESOURCE_UUID: Uuid = uuid!("692fec79-103a-483c-bb0b-9fc3a349cb48");

/// A trait for resource data.
pub trait ResourceData: Debug + Visit + Send + Reflect {
    /// Returns unique data type id.
    fn type_uuid(&self) -> Uuid;

    /// Saves the resource data a file at the specified path. This method is free to
    /// decide how the resource data is saved. This is needed, because there are multiple formats
    /// that defines various kinds of resources. For example, a rectangular texture could be saved
    /// into a bunch of formats, such as png, bmp, tga, jpg etc., but in the engine it is single
    /// Texture resource. In any case, produced file should be compatible with a respective resource
    /// loader.
    fn save(&mut self, #[allow(unused_variables)] path: &Path) -> Result<(), Box<dyn Error>>;

    /// Returns `true` if the resource data can be saved to a file, `false` - otherwise. Not every
    /// resource type supports saving, for example there might be temporary resource type that is
    /// used only at runtime which does not need saving at all.
    fn can_be_saved(&self) -> bool;

    /// Tries to clone the resource data. This method can return `None` if the underlying type is
    /// non-cloneable.
    fn try_clone_box(&self) -> Option<Box<dyn ResourceData>>;
}

/// Extension trait for a resource data of a particular type, which adds additional functionality,
/// such as: a way to get default state of the data (`Default` impl), a way to get data's type uuid.
/// The trait has automatic implementation for any type that implements
/// ` ResourceData + Default + TypeUuidProvider` traits.
pub trait TypedResourceData: ResourceData + Default + TypeUuidProvider {}

impl<T> TypedResourceData for T where T: ResourceData + Default + TypeUuidProvider {}

/// A trait for resource load error.
pub trait ResourceLoadError: 'static + Debug + Display + Send + Sync {}

impl<T> ResourceLoadError for T where T: 'static + Debug + Display + Send + Sync {}

/// Provides typed access to a resource state.
pub struct ResourceHeaderGuard<'a, T>
where
    T: TypedResourceData,
{
    guard: MutexGuard<'a, ResourceHeader>,
    phantom: PhantomData<T>,
}

impl<'a, T> From<MutexGuard<'a, ResourceHeader>> for ResourceHeaderGuard<'a, T>
where
    T: TypedResourceData,
{
    fn from(guard: MutexGuard<'a, ResourceHeader>) -> Self {
        Self {
            guard,
            phantom: PhantomData,
        }
    }
}

impl<T> ResourceHeaderGuard<'_, T>
where
    T: TypedResourceData,
{
    /// The UUID that universally identifies the resource.
    pub fn resource_uuid(&self) -> Uuid {
        self.guard.uuid
    }

    /// Returns resource kind of the locked resource.
    pub fn kind(&self) -> ResourceKind {
        self.guard.kind
    }

    /// Tries to fetch the underlying data of the resource type. This operation will fail if the
    /// locked resource is not in [`ResourceState::Ok`] or if its actual data does not match the
    /// type of the resource.
    pub fn data(&mut self) -> Option<&mut T> {
        if let ResourceState::Ok { ref mut data, .. } = self.guard.state {
            (&mut **data as &mut dyn Any).downcast_mut::<T>()
        } else {
            None
        }
    }

    /// Tries to fetch the underlying data of the resource type. This operation will fail if the
    /// locked resource is not in [`ResourceState::Ok`] or if its actual data does not match the
    /// type of the resource.
    pub fn data_ref(&self) -> Option<&T> {
        if let ResourceState::Ok { ref data, .. } = self.guard.state {
            (&**data as &dyn Any).downcast_ref::<T>()
        } else {
            None
        }
    }

    /// Tries to fetch the underlying data of the resource type. This operation will fail if the
    /// locked resource is not in [`ResourceState::Ok`] or if its actual data does not match the
    /// type of the resource.
    pub fn data_ref_with_id(&self) -> Option<(&T, &Uuid)> {
        let uuid = &self.guard.uuid;
        if let ResourceState::Ok { ref data } = self.guard.state {
            (&**data as &dyn Any)
                .downcast_ref::<T>()
                .map(|typed| (typed, uuid))
        } else {
            None
        }
    }
}

/// A resource of particular data type. It is a typed wrapper around [`UntypedResource`] which
/// does type checks at runtime. See [`UntypedResource`] for more info.
///
/// ## Default State
///
/// Default state of the resource will be [`ResourceState::Ok`] with `T::default`.
#[derive(Reflect, Serialize, Deserialize)]
#[serde(
    bound = "T: TypedResourceData",
    from = "UntypedResource",
    into = "UntypedResource"
)]
pub struct Resource<T: Debug> {
    untyped: UntypedResource,
    #[reflect(hidden)]
    phantom: PhantomData<T>,
}

impl<T: Debug> Debug for Resource<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Resource<{}>({})",
            std::any::type_name::<T>(),
            self.untyped
        )
    }
}

impl<T: TypedResourceData> AsRef<UntypedResource> for Resource<T> {
    fn as_ref(&self) -> &UntypedResource {
        &self.untyped
    }
}

impl AsRef<Self> for UntypedResource {
    fn as_ref(&self) -> &Self {
        self
    }
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
        self.untyped
            .visit_with_type_uuid(name, Some(<T as TypeUuidProvider>::type_uuid()), visitor)
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
    /// Create a summary of this resource handle, wht UUID, embedded/external, and Ok, Pending, Error, etc.
    pub fn summary(&self) -> String {
        format!("{}", self.untyped)
    }
    /// Creates new resource in pending state.
    #[inline]
    pub fn new_pending(uuid: Uuid, kind: ResourceKind) -> Self {
        Self {
            untyped: UntypedResource::new_pending(uuid, kind),
            phantom: PhantomData,
        }
    }

    /// Creates new resource in ok state (fully loaded).
    #[inline]
    pub fn new_ok(resource_uuid: Uuid, kind: ResourceKind, data: T) -> Self {
        Self {
            untyped: UntypedResource::new_ok(resource_uuid, kind, data),
            phantom: PhantomData,
        }
    }

    /// Creates a new embedded resource in ok state (fully loaded).
    #[inline]
    pub fn new_embedded(data: T) -> Self {
        Self {
            untyped: UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::Embedded, data),
            phantom: PhantomData,
        }
    }

    /// Creates new resource in error state.
    #[inline]
    pub fn new_load_error(kind: ResourceKind, path: PathBuf, error: LoadError) -> Self {
        Self {
            untyped: UntypedResource::new_load_error(kind, path, error),
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
        self.untyped.typed_lock()
    }

    /// Tries to lock internal mutex provides access to the state.
    #[inline]
    pub fn try_acquire_state(&self) -> Option<ResourceHeaderGuard<'_, T>> {
        self.untyped.try_typed_lock()
    }

    /// Locks the resource and provides access to its header. See [`ResourceHeader`] docs for more info.
    #[inline]
    pub fn header(&self) -> MutexGuard<'_, ResourceHeader> {
        self.untyped.lock()
    }

    /// Returns true if the resource is still loading.
    #[inline]
    pub fn is_loading(&self) -> bool {
        self.untyped.is_loading()
    }

    /// Returns true if the resource is fully loaded and ready for use.
    #[inline]
    pub fn is_ok(&self) -> bool {
        self.untyped.is_ok()
    }

    /// Returns true if the resource is failed to load.
    #[inline]
    pub fn is_failed_to_load(&self) -> bool {
        self.untyped.is_failed_to_load()
    }

    /// Returns exact amount of users of the resource.
    #[inline]
    pub fn use_count(&self) -> usize {
        self.untyped.use_count()
    }

    /// Returns a pointer as numeric value which can be used as a hash.
    #[inline]
    pub fn key(&self) -> u64 {
        self.untyped.key()
    }

    /// Returns kind of the resource, if the resource is registered.
    #[inline]
    pub fn kind(&self) -> ResourceKind {
        self.untyped.kind()
    }

    /// The UUID of the resource. All resources must have a UUID, even if they are not loaded
    /// because the UUID is how the resource manager knows the path to load from.
    #[inline]
    pub fn resource_uuid(&self) -> Uuid {
        self.untyped.resource_uuid()
    }

    /// Sets a new kind of the resource.
    #[inline]
    pub fn set_path(&mut self, new_kind: ResourceKind) {
        self.untyped.set_kind(new_kind);
    }

    /// Allows you to get a reference to the resource data. The returned object implements [`Deref`]
    /// and [`DerefMut`] traits, and basically acts like a reference to the resource value.
    ///
    /// # Panic
    ///
    /// An attempt to dereference the returned object will result in panic if the resource is not
    /// loaded yet, or there was a loading error. Usually this is ok because you should chain this
    /// call like this `resource.await?.data_ref()`. Every resource implements [`Future`] trait,
    /// and it returns Result, so if you'll await the future then you'll get Result, so call to
    /// `data_ref` will be fine.
    ///
    /// You can also use [`ResourceDataRef::as_loaded_ref`] and [`ResourceDataRef::as_loaded_mut`]
    /// methods that perform checked access to the resource internals.
    #[inline]
    pub fn data_ref(&self) -> ResourceDataRef<'_, T> {
        ResourceDataRef {
            guard: self.untyped.lock(),
            phantom: Default::default(),
        }
    }

    /// Tries to save the resource to the specified path.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        self.untyped.save(path)
    }
}

impl<T> Default for Resource<T>
where
    T: TypedResourceData,
{
    #[inline]
    fn default() -> Self {
        Self {
            untyped: UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::Embedded, T::default()),
            phantom: Default::default(),
        }
    }
}

impl<T: Debug> Clone for Resource<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            untyped: self.untyped.clone(),
            phantom: Default::default(),
        }
    }
}

impl<T> From<Uuid> for Resource<T>
where
    T: TypedResourceData,
{
    fn from(uuid: Uuid) -> Self {
        UntypedResource::from(uuid).into()
    }
}

impl<T> From<UntypedResource> for Resource<T>
where
    T: TypedResourceData,
{
    #[inline]
    fn from(untyped: UntypedResource) -> Self {
        if let Some(type_uuid) = untyped.type_uuid() {
            let expected = <T as TypeUuidProvider>::type_uuid();
            if type_uuid != expected {
                panic!("Resource type mismatch. Expected: {expected}. Found: {type_uuid}");
            }
        }
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

impl<T> ResourceDataRef<'_, T>
where
    T: TypedResourceData,
{
    #[inline]
    pub fn as_loaded_ref(&self) -> Option<&T> {
        match self.guard.state {
            ResourceState::Ok { ref data, .. } => (&**data as &dyn Any).downcast_ref(),
            _ => None,
        }
    }

    #[inline]
    pub fn as_loaded_mut(&mut self) -> Option<&mut T> {
        match self.guard.state {
            ResourceState::Ok { ref mut data, .. } => (&mut **data as &mut dyn Any).downcast_mut(),
            _ => None,
        }
    }
}

impl<T> Debug for ResourceDataRef<'_, T>
where
    T: TypedResourceData,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.guard.state {
            ResourceState::Unloaded => {
                write!(
                    f,
                    "Attempt to get reference to resource data while it is unloaded!"
                )
            }
            ResourceState::Pending { .. } => {
                write!(
                    f,
                    "Attempt to get reference to resource data while it is loading!"
                )
            }
            ResourceState::LoadError { .. } => {
                write!(
                    f,
                    "Attempt to get reference to resource data which failed to load!"
                )
            }
            ResourceState::Ok { ref data, .. } => data.fmt(f),
        }
    }
}

impl<T> Deref for ResourceDataRef<'_, T>
where
    T: TypedResourceData,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self.guard.state {
            ResourceState::Unloaded => {
                panic!(
                    "Attempt to get reference to resource data while it is unloaded! Type {}",
                    std::any::type_name::<T>()
                )
            }
            ResourceState::Pending { .. } => {
                panic!(
                    "Attempt to get reference to resource data while it is loading! Type {}",
                    std::any::type_name::<T>()
                )
            }
            ResourceState::LoadError {
                ref path,
                ref error,
            } => {
                let path = if path.as_os_str().is_empty() {
                    "Unknown".to_string()
                } else {
                    format!("{path:?}")
                };
                panic!("Attempt to get reference to resource data which failed to load! Type {}. Path: {path}. Error: {error:?}", std::any::type_name::<T>())
            }
            ResourceState::Ok { ref data } => (&**data as &dyn Any)
                .downcast_ref()
                .expect("Type mismatch!"),
        }
    }
}

impl<T> DerefMut for ResourceDataRef<'_, T>
where
    T: TypedResourceData,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        let header = &mut *self.guard;
        match header.state {
            ResourceState::Unloaded => {
                panic!("Attempt to get reference to resource data while it is unloaded!")
            }
            ResourceState::Pending { .. } => {
                panic!("Attempt to get reference to resource data while it is loading!")
            }
            ResourceState::LoadError { .. } => {
                panic!("Attempt to get reference to resource data which failed to load!")
            }
            ResourceState::Ok { ref mut data, .. } => (&mut **data as &mut dyn Any)
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

    entity.fields_ref(&mut |fields| {
        for field in fields {
            collect_used_resources(field.value.field_value_as_reflect(), resources_collection);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        io::{FsResourceIo, ResourceIo},
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader, ResourceLoadersContainer},
        manager::ResourceManager,
        metadata::ResourceMetadata,
        registry::ResourceRegistry,
        state::LoadError,
        ResourceData,
    };
    use fyrox_core::{
        append_extension, futures::executor::block_on, io::FileError, parking_lot::Mutex,
        reflect::prelude::*, task::TaskPool, uuid, visitor::prelude::*, SafeLock, TypeUuidProvider,
        Uuid,
    };
    use ron::ser::PrettyConfig;
    use serde::{Deserialize, Serialize};
    use std::{
        error::Error,
        fs::File,
        io::Write,
        ops::Range,
        path::{Path, PathBuf},
        sync::Arc,
    };

    #[derive(Serialize, Deserialize, Default, Debug, Clone, Visit, Reflect, TypeUuidProvider)]
    #[type_uuid(id = "241d14c7-079e-4395-a63c-364f0fc3e6ea")]
    struct MyData {
        data: u32,
    }

    impl MyData {
        pub async fn load_from_file(
            path: &Path,
            resource_io: &dyn ResourceIo,
        ) -> Result<Self, FileError> {
            resource_io.load_file(path).await.and_then(|metadata| {
                ron::de::from_bytes::<Self>(&metadata).map_err(|err| {
                    FileError::Custom(format!(
                        "Unable to deserialize the resource metadata. Reason: {err:?}"
                    ))
                })
            })
        }
    }

    impl ResourceData for MyData {
        fn type_uuid(&self) -> Uuid {
            <Self as TypeUuidProvider>::type_uuid()
        }

        fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
            let string = ron::ser::to_string_pretty(self, PrettyConfig::default())
                .map_err(|err| {
                    FileError::Custom(format!(
                        "Unable to serialize resource metadata for {} resource! Reason: {}",
                        path.display(),
                        err
                    ))
                })
                .map_err(|_| "error".to_string())?;
            let mut file = File::create(path)?;
            file.write_all(string.as_bytes())?;
            Ok(())
        }

        fn can_be_saved(&self) -> bool {
            true
        }

        fn try_clone_box(&self) -> Option<Box<dyn ResourceData>> {
            Some(Box::new(self.clone()))
        }
    }

    struct MyDataLoader {}

    impl MyDataLoader {
        const EXT: &'static str = "my_data";
    }

    impl ResourceLoader for MyDataLoader {
        fn extensions(&self) -> &[&str] {
            &[Self::EXT]
        }

        fn data_type_uuid(&self) -> Uuid {
            <MyData as TypeUuidProvider>::type_uuid()
        }

        fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
            Box::pin(async move {
                let my_data = MyData::load_from_file(&path, io.as_ref())
                    .await
                    .map_err(LoadError::new)?;
                Ok(LoaderPayload::new(my_data))
            })
        }
    }

    const TEST_FOLDER1: &str = "./test_output1";
    const TEST_FOLDER2: &str = "./test_output2";
    const TEST_FOLDER3: &str = "./test_output3";

    fn make_file_path(root: &str, n: usize) -> PathBuf {
        Path::new(root).join(format!("test{n}.{}", MyDataLoader::EXT))
    }

    fn make_metadata_file_path(root: &str, n: usize) -> PathBuf {
        Path::new(root).join(format!(
            "test{n}.{}.{}",
            MyDataLoader::EXT,
            ResourceMetadata::EXTENSION
        ))
    }

    fn write_test_resources(root: &str, indices: Range<usize>) {
        let path = Path::new(root);
        if !std::fs::exists(path).unwrap() {
            std::fs::create_dir_all(path).unwrap();
        }

        for i in indices {
            MyData { data: i as u32 }
                .save(&make_file_path(root, i))
                .unwrap();
        }
    }

    #[test]
    fn test_serialize() {
        let uuid = uuid!("6d1aadb5-42e1-485b-910b-fa4d81b61855");
        let typed = Resource::<MyData>::from(uuid);
        let untyped = UntypedResource::from(uuid);
        let s = ron::ser::to_string(&typed).unwrap();
        assert_eq!(&ron::ser::to_string(&uuid).unwrap(), &s);
        assert_eq!(&ron::ser::to_string(&untyped).unwrap(), &s);
        let output_uuid = ron::de::from_str::<Uuid>(&s).unwrap();
        assert_eq!(output_uuid, uuid);
        let untyped = ron::de::from_str::<UntypedResource>(&s).unwrap();
        assert_eq!(untyped.resource_uuid(), uuid);
        assert_eq!(untyped.kind(), ResourceKind::External);
        let output = ron::de::from_str::<Resource<MyData>>(&s).unwrap();
        assert_eq!(output.resource_uuid(), uuid);
        assert_eq!(output.kind(), ResourceKind::External);
    }

    #[test]
    fn test_registry_scan() {
        write_test_resources(TEST_FOLDER1, 0..2);

        assert!(std::fs::exists(make_file_path(TEST_FOLDER1, 0)).unwrap());
        assert!(std::fs::exists(make_file_path(TEST_FOLDER1, 1)).unwrap());

        let io = Arc::new(FsResourceIo);

        let mut loaders = ResourceLoadersContainer::new();
        loaders.set(MyDataLoader {});
        let loaders = Arc::new(Mutex::new(loaders));

        let registry = block_on(ResourceRegistry::scan(
            io,
            loaders,
            Path::new(TEST_FOLDER1).join("resources.registry"),
            Default::default(),
        ));

        assert!(std::fs::exists(make_metadata_file_path(TEST_FOLDER1, 0)).unwrap());
        assert!(std::fs::exists(make_metadata_file_path(TEST_FOLDER1, 1)).unwrap());

        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_resource_manager_request_simple() {
        write_test_resources(TEST_FOLDER2, 2..4);
        let resource_manager =
            ResourceManager::new(Arc::new(FsResourceIo), Arc::new(TaskPool::new()));
        resource_manager
            .state()
            .resource_registry
            .safe_lock()
            .set_path(Path::new(TEST_FOLDER2).join("resources.registry"));
        resource_manager.add_loader(MyDataLoader {});
        resource_manager.update_or_load_registry();
        let path1 = make_file_path(TEST_FOLDER2, 2);
        let path2 = make_file_path(TEST_FOLDER2, 3);
        let res1 = resource_manager.request::<MyData>(&path1);
        let res1_2 = resource_manager.request::<MyData>(&path1);
        assert_eq!(res1.key(), res1_2.key());
        let res2 = resource_manager.request::<MyData>(path2);
        assert_ne!(res1.key(), res2.key());
        assert_eq!(block_on(res1).unwrap().data_ref().data, 2);
        assert_eq!(block_on(res2).unwrap().data_ref().data, 3);
    }

    #[test]
    fn test_move_resource() {
        write_test_resources(TEST_FOLDER3, 0..2);
        let resource_manager =
            ResourceManager::new(Arc::new(FsResourceIo), Arc::new(TaskPool::new()));
        resource_manager
            .state()
            .resource_registry
            .safe_lock()
            .set_path(Path::new(TEST_FOLDER3).join("resources.registry"));
        resource_manager.add_loader(MyDataLoader {});
        resource_manager.update_or_load_registry();
        let path1 = make_file_path(TEST_FOLDER3, 0);
        let path2 = make_file_path(TEST_FOLDER3, 1);
        let res1 = resource_manager.request::<MyData>(path1);
        let res2 = resource_manager.request::<MyData>(path2);
        assert_eq!(block_on(res1.clone()).unwrap().data_ref().data, 0);
        assert_eq!(block_on(res2.clone()).unwrap().data_ref().data, 1);
        let new_res1_path = ResourceRegistry::normalize_path(make_file_path(TEST_FOLDER3, 3));
        let new_res2_path = ResourceRegistry::normalize_path(make_file_path(TEST_FOLDER3, 4));
        block_on(resource_manager.move_resource(res1.as_ref(), &new_res1_path, true)).unwrap();
        block_on(resource_manager.move_resource(res2.as_ref(), &new_res2_path, true)).unwrap();
        assert_eq!(
            resource_manager.resource_path(res1.as_ref()).unwrap(),
            new_res1_path
        );
        assert!(
            std::fs::exists(append_extension(new_res1_path, ResourceMetadata::EXTENSION)).unwrap()
        );
        assert_eq!(
            resource_manager.resource_path(res2.as_ref()).unwrap(),
            new_res2_path
        );
        assert!(
            std::fs::exists(append_extension(new_res2_path, ResourceMetadata::EXTENSION)).unwrap()
        );
    }
}
