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

//! Resource manager controls loading and lifetime of resource in the engine. See [`ResourceManager`]
//! docs for more info.

pub use crate::builtin::*;
use crate::{
    constructor::ResourceConstructorContainer,
    core::{
        append_extension, err,
        futures::future::join_all,
        info,
        io::FileError,
        log::Log,
        notify, ok_or_continue,
        parking_lot::{Mutex, MutexGuard},
        task::TaskPool,
        watcher::FileSystemWatcher,
        SafeLock, TypeUuidProvider, Uuid,
    },
    entry::{TimedEntry, DEFAULT_RESOURCE_LIFETIME},
    event::{ResourceEvent, ResourceEventBroadcaster},
    io::ResourceIo,
    loader::{ResourceLoader, ResourceLoadersContainer},
    metadata::ResourceMetadata,
    options::OPTIONS_EXTENSION,
    registry::{RegistryUpdate, ResourceRegistry, ResourceRegistryRefMut, ResourceRegistryStatus},
    state::{LoadError, ResourceDataWrapper, ResourceState},
    untyped::ResourceKind,
    Resource, TypedResourceData, UntypedResource,
};
use fxhash::FxHashSet;
use fyrox_core::{
    futures::executor::block_on, make_relative_path, notify::Event, ok_or_return, some_or_continue,
    some_or_return,
};
use std::{
    fmt::{Debug, Display, Formatter},
    io::Error,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

/// A set of resources that can be waited for.
#[must_use]
#[derive(Default)]
pub struct ResourceWaitContext {
    resources: Vec<UntypedResource>,
}

impl ResourceWaitContext {
    /// Wait until all resources are loaded (or failed to load).
    #[must_use]
    pub fn is_all_loaded(&self) -> bool {
        for resource in self.resources.iter() {
            if resource.is_loading() {
                return false;
            }
        }
        true
    }
}

/// Internal state of the resource manager.
pub struct ResourceManagerState {
    /// A set of resource loaders. Use this field to register your own resource loader.
    pub loaders: Arc<Mutex<ResourceLoadersContainer>>,
    /// Event broadcaster can be used to "subscribe" for events happening inside the container.
    pub event_broadcaster: ResourceEventBroadcaster,
    /// A container for resource constructors.
    pub constructors_container: ResourceConstructorContainer,
    /// A set of built-in resources, that will be used to resolve references on deserialization.
    pub built_in_resources: BuiltInResourcesContainer,
    /// File system abstraction interface. Could be used to support virtual file systems.
    pub resource_io: Arc<dyn ResourceIo>,
    /// Resource registry, contains associations `UUID -> File Path`. Any access to the registry
    /// must be async, use task pool for this.
    pub resource_registry: Arc<Mutex<ResourceRegistry>>,

    resources: Vec<TimedEntry<UntypedResource>>,
    task_pool: Arc<TaskPool>,
    watcher: Option<FileSystemWatcher>,
}

/// Resource manager controls loading and lifetime of resource in the engine. Resource manager can hold
/// resources of arbitrary types via type erasure mechanism.
///
/// ## Built-in Resources
///
/// Built-in resources are special kinds of resources, whose data is packed in the executable (i.e. via
/// [`include_bytes`] macro). Such resources reference the data that cannot be "loaded" from external
/// source. To support such kind of resource the manager provides `built_in_resources` hash map where
/// you can register your own built-in resource and access existing ones.
///
/// ## Internals
///
/// It is a simple wrapper over [`ResourceManagerState`] that can be shared (cloned). In other words,
/// it is just a strong reference to the inner state.
#[derive(Clone)]
pub struct ResourceManager {
    state: Arc<Mutex<ResourceManagerState>>,
}

impl Debug for ResourceManager {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResourceManager")
    }
}

/// An error that may occur during texture registration.
#[derive(Debug, PartialEq, Eq)]
pub enum ResourceRegistrationError {
    /// Resource saving has failed.
    UnableToRegister,
    /// Resource was in invalid state (Pending, LoadErr)
    InvalidState,
    /// Resource is already registered.
    AlreadyRegistered,
    /// An error occurred on an attempt to write resource metadata.
    UnableToCreateMetadata,
}

impl Display for ResourceRegistrationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceRegistrationError::UnableToRegister => {
                write!(f, "Unable to register the resource!")
            }
            ResourceRegistrationError::InvalidState => {
                write!(f, "A resource was in invalid state!")
            }
            ResourceRegistrationError::AlreadyRegistered => {
                write!(f, "A resource is already registered!")
            }
            ResourceRegistrationError::UnableToCreateMetadata => {
                write!(
                    f,
                    "An error occurred on an attempt to write resource metadata!"
                )
            }
        }
    }
}

/// All the required and validated data that is needed to move a resource from the path A to the path B.
pub struct ResourceMoveContext {
    relative_src_path: PathBuf,
    relative_dest_path: PathBuf,
    resource_uuid: Uuid,
}

/// A possible set of errors that may occur during resource movement.
#[derive(Debug)]
pub enum ResourceMovementError {
    /// An IO error.
    Io(std::io::Error),
    /// A file error.
    FileError(FileError),
    /// The resource at the `src_path` already exist at the `dest_path`.
    AlreadyExist {
        /// Source path of the resource.
        src_path: PathBuf,
        /// The path at which a resource with the same name is located.
        dest_path: PathBuf,
    },
    /// The new path for a resource is invalid.
    DestinationPathIsInvalid {
        /// Source path of the resource.
        src_path: PathBuf,
        /// The invalid destination path.
        dest_path: PathBuf,
    },
    /// Resource registry location is unknown (the registry wasn't saved yet).
    ResourceRegistryLocationUnknown {
        /// A path of the resource being moved.
        resource_path: PathBuf,
    },
    /// The resource is not in the registry.
    NotInRegistry {
        /// A path of the resource being moved.
        resource_path: PathBuf,
    },
    /// Attempting to move a resource outside the registry.
    OutsideOfRegistry {
        /// An absolute path of the resource being moved.
        absolute_src_path: PathBuf,
        /// An absolute path of the destination folder.
        absolute_dest_dir: PathBuf,
        /// An absolute path of the resource registry.
        absolute_registry_dir: PathBuf,
    },
    /// A resource has no path. It is either an embedded resource or in an invalid
    /// state (failed to load or still loading).
    NoPath(UntypedResource),
}

impl From<FileError> for ResourceMovementError {
    fn from(value: FileError) -> Self {
        Self::FileError(value)
    }
}

impl From<std::io::Error> for ResourceMovementError {
    fn from(value: Error) -> Self {
        Self::Io(value)
    }
}

impl Display for ResourceMovementError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceMovementError::Io(err) => {
                write!(f, "Io error: {err}")
            }
            ResourceMovementError::FileError(err) => {
                write!(f, "File error: {err}")
            }
            ResourceMovementError::AlreadyExist {
                src_path,
                dest_path,
            } => {
                write!(
                    f,
                    "Unable to move the {} resource, because the destination \
                    path {} points to an existing file!",
                    src_path.display(),
                    dest_path.display()
                )
            }
            ResourceMovementError::DestinationPathIsInvalid {
                src_path,
                dest_path,
            } => {
                write!(
                    f,
                    "Unable to move the {} resource, because the destination \
                    path {} is invalid!",
                    src_path.display(),
                    dest_path.display()
                )
            }
            ResourceMovementError::ResourceRegistryLocationUnknown { resource_path } => {
                write!(
                    f,
                    "Unable to move the {} resource, because the registry location is unknown!",
                    resource_path.display()
                )
            }
            ResourceMovementError::NotInRegistry { resource_path } => {
                write!(
                    f,
                    "Unable to move the {} resource, because it is not in the registry!",
                    resource_path.display()
                )
            }
            ResourceMovementError::OutsideOfRegistry {
                absolute_src_path,
                absolute_dest_dir,
                absolute_registry_dir,
            } => {
                write!(
                    f,
                    "Unable to move the {} resource to {} path, because \
            the new path is located outside the resource registry path {}!",
                    absolute_src_path.display(),
                    absolute_dest_dir.display(),
                    absolute_registry_dir.display()
                )
            }
            ResourceMovementError::NoPath(resource) => {
                write!(
                    f,
                    "Unable to move {} resource, because it does not have a \
                file system path!",
                    resource.key()
                )
            }
        }
    }
}

/// A set of potential errors that may occur when moving a folder with resources.
#[derive(Debug)]
pub enum FolderMovementError {
    /// An IO error.
    Io(std::io::Error),
    /// A file error.
    FileError(FileError),
    /// A [`ResourceMovementError`].
    ResourceMovementError(ResourceMovementError),
    /// The folder is not in the registry.
    NotInRegistry {
        /// A path of the folder being moved.
        dest_dir: PathBuf,
    },
    /// Trying to move a folder into one of its own sub-folders.
    HierarchyError {
        /// Path of the folder being moved.
        src_dir: PathBuf,
        /// Destination directory.
        dest_dir: PathBuf,
    },
}

impl From<FileError> for FolderMovementError {
    fn from(value: FileError) -> Self {
        Self::FileError(value)
    }
}

impl From<std::io::Error> for FolderMovementError {
    fn from(value: Error) -> Self {
        Self::Io(value)
    }
}

impl From<ResourceMovementError> for FolderMovementError {
    fn from(value: ResourceMovementError) -> Self {
        Self::ResourceMovementError(value)
    }
}

impl Display for FolderMovementError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FolderMovementError::Io(err) => {
                write!(f, "Io error: {err}")
            }
            FolderMovementError::FileError(err) => {
                write!(f, "File error: {err}")
            }
            FolderMovementError::ResourceMovementError(err) => {
                write!(f, "{err}")
            }
            FolderMovementError::NotInRegistry { dest_dir } => {
                write!(
                    f,
                    "Unable to move the {} folder, because it is not in the registry!",
                    dest_dir.display()
                )
            }
            FolderMovementError::HierarchyError { src_dir, dest_dir } => {
                write!(
                    f,
                    "Trying to move a folder into one of its own sub-folders. \
                    Source folder is {}, destination folder is {}",
                    src_dir.display(),
                    dest_dir.display()
                )
            }
        }
    }
}

impl ResourceManager {
    /// Creates a resource manager with default settings and loaders.
    pub fn new(io: Arc<dyn ResourceIo>, task_pool: Arc<TaskPool>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ResourceManagerState::new(io, task_pool))),
        }
    }

    /// Returns a guarded reference to the internal state of resource manager. This is blocking
    /// method and it may deadlock if used incorrectly (trying to get the state lock one more time
    /// when there's an existing lock in the same thread, multi-threading-related deadlock and so on).
    pub fn state(&self) -> MutexGuard<'_, ResourceManagerState> {
        self.state.safe_lock()
    }

    /// Returns a guarded reference to the internal state of resource manager. This method will try
    /// to acquire the state lock for the given time and if it fails, returns `None`.
    pub fn try_get_state(&self, timeout: Duration) -> Option<MutexGuard<'_, ResourceManagerState>> {
        self.state.try_lock_for(timeout)
    }

    /// Returns the ResourceIo used by this resource manager
    pub fn resource_io(&self) -> Arc<dyn ResourceIo> {
        let state = self.state();
        state.resource_io.clone()
    }

    /// Returns the task pool used by this resource manager.
    pub fn task_pool(&self) -> Arc<TaskPool> {
        let state = self.state();
        state.task_pool()
    }

    /// Registers a new built-in resource, so it becomes accessible via [`Self::request`].
    pub fn register_built_in_resource<T: TypedResourceData>(
        &self,
        resource: BuiltInResource<T>,
    ) -> Option<UntypedBuiltInResource> {
        self.state().register_built_in_resource(resource)
    }

    /// The same as [`Self::find`], but returns [`None`] if type UUID of `T` does not match the actual type UUID
    /// of the resource.
    ///
    /// ## Panic
    ///
    /// This method does not panic.
    pub fn try_find<T>(&self, path: impl AsRef<Path>) -> Option<Resource<T>>
    where
        T: TypedResourceData,
    {
        let mut state = self.state();

        let untyped = state.find(path.as_ref());

        let data_type_uuid_matches = untyped
            .type_uuid_non_blocking()
            .is_some_and(|uuid| uuid == <T as TypeUuidProvider>::type_uuid());

        if !data_type_uuid_matches {
            let has_loader_for_extension = state
                .loaders
                .safe_lock()
                .is_extension_matches_type::<T>(path.as_ref());

            if !has_loader_for_extension {
                return None;
            }
        }

        Some(Resource {
            untyped,
            phantom: PhantomData::<T>,
        })
    }

    /// Find the resource for the given path without loading the resource.
    /// A new unloaded resource is created if one does not already exist.
    /// If the path is not in the registry then a new UUID is generated and
    /// the path is added to the registry.
    /// If the resource is not already loading, then it will be returned in
    /// the [`ResourceState::Unloaded`] state, and [`Self::request_resource`] may
    /// be used to begin the loading process.
    ///
    /// ## Panic
    ///
    /// This method will panic if type UUID of `T` does not match the actual type UUID of the resource. If this
    /// is undesirable, use [`Self::try_find`] instead.
    pub fn find<T>(&self, path: impl AsRef<Path>) -> Resource<T>
    where
        T: TypedResourceData,
    {
        let path = path.as_ref();
        let mut state = self.state();

        let untyped = state.find(path);

        let data_type_uuid_matches = untyped
            .type_uuid_non_blocking()
            .is_some_and(|uuid| uuid == <T as TypeUuidProvider>::type_uuid());

        if !data_type_uuid_matches {
            let has_loader_for_extension = state
                .loaders
                .safe_lock()
                .is_extension_matches_type::<T>(path);

            if !has_loader_for_extension {
                panic!(
                    "Unable to get a resource of type {} from {path:?}! The resource has no \
                    associated loader for its extension and its actual data has some other \
                    data type!",
                    <T as TypeUuidProvider>::type_uuid(),
                )
            }
        }

        Resource {
            untyped,
            phantom: PhantomData::<T>,
        }
    }

    /// Find the resource for the given UUID without loading the resource.
    /// If the resource is not already loading, then it will be returned in
    /// the [`ResourceState::Unloaded`] state, and [`Self::request_resource`] may
    /// be used to begin the loading process.
    ///
    /// ## Panic
    ///
    /// This method will panic if type UUID of `T` does not match the actual type UUID of the resource. If this
    /// is undesirable, use [`Self::try_find`] instead.
    pub fn find_uuid<T>(&self, uuid: Uuid) -> Resource<T>
    where
        T: TypedResourceData,
    {
        let mut state = self.state();

        let untyped = state.find_uuid(uuid);

        if let Some(type_uuid) = untyped.type_uuid_non_blocking() {
            if type_uuid != <T as TypeUuidProvider>::type_uuid() {
                panic!(
                    "Unable to get a resource of type {} from {uuid} UUID! Its actual data has some other \
                    data type with type UUID {type_uuid}!",
                    <T as TypeUuidProvider>::type_uuid(),
                )
            }
        }

        Resource {
            untyped,
            phantom: PhantomData::<T>,
        }
    }

    /// Requests a resource of the given type located at the given path. This method is non-blocking, instead
    /// it immediately returns the typed resource wrapper. Loading of the resource is managed automatically in
    /// a separate thread (or thread pool) on PC, and JS micro-task (the same thread) on WebAssembly.
    ///
    /// ## Type Guarantees
    ///
    /// There's no strict guarantees that the requested resource will be of the requested type. This
    /// is because the resource system is fully async and does not have access to type information in
    /// most cases. Initial type checking is not very reliable and can be "fooled" pretty easily,
    /// simply because it just checks if there's a registered loader for a specific extension.
    ///
    /// ## Sharing
    ///
    /// If the resource at the given path is already was requested (no matter in which state the actual resource
    /// is), this method will return the existing instance. This way the resource manager guarantees that the actual
    /// resource data will be loaded once, and it can be shared.
    ///
    /// ## Waiting
    ///
    /// If you need to wait until the resource is loaded, use `.await` on the result of the method. Every resource
    /// implements `Future` trait and can be used in `async` contexts.
    ///
    /// ## Resource state
    ///
    /// Keep in mind, that the resource itself is a small state machine. It could be in three main states:
    ///
    /// - [`ResourceState::Pending`] - a resource is in the queue to load or still loading.
    /// - [`ResourceState::LoadError`] - a resource is failed to load.
    /// - [`ResourceState::Ok`] - a resource is successfully loaded.
    ///
    /// Actual resource state can be fetched by [`Resource::state`] method. If you know for sure that the resource
    /// is already loaded, then you can use [`Resource::data_ref`] to obtain a reference to the actual resource data.
    /// Keep in mind, that this method will panic if the resource non in `Ok` state.
    ///
    /// ## Panic
    ///
    /// This method will panic, if type UUID of `T` does not match the actual type UUID of the resource. If this
    /// is undesirable, use [`Self::try_request`] instead.
    pub fn request<T>(&self, path: impl AsRef<Path>) -> Resource<T>
    where
        T: TypedResourceData,
    {
        let path = path.as_ref();
        let mut state = self.state();

        let untyped = state.request(path);

        let data_type_uuid_matches = untyped
            .type_uuid_non_blocking()
            .is_some_and(|uuid| uuid == <T as TypeUuidProvider>::type_uuid());

        if !data_type_uuid_matches {
            let has_loader_for_extension = state
                .loaders
                .safe_lock()
                .is_extension_matches_type::<T>(path);

            if !has_loader_for_extension {
                panic!(
                    "Unable to get a resource of type {} from {path:?}! The resource has no \
                    associated loader for its extension and its actual data has some other \
                    data type!",
                    <T as TypeUuidProvider>::type_uuid(),
                )
            }
        }

        Resource {
            untyped,
            phantom: PhantomData::<T>,
        }
    }

    /// Requests that given resource should begin loading, if not already loading or loaded.
    /// This method is non-blocking, instead it modifies the given resource and returns.
    /// Loading of the resource is managed automatically in a separate thread (or thread pool) on PC,
    /// and JS micro-task (the same thread) on WebAssembly.
    ///
    /// ## Type Guarantees
    ///
    /// There's no strict guarantees that the requested resource will be of the requested type. This
    /// is because the resource system is fully async and does not have access to type information in
    /// most cases. Initial type checking is not very reliable and can be "fooled" pretty easily,
    /// simply because it just checks if there's a registered loader for a specific extension.
    ///
    /// ## Sharing
    ///
    /// If a resource with given resource's UUID was already requested (no matter in which state the actual resource
    /// is), this method will modify the given resource to be a shared reference to the already loading resource.
    /// This way the resource manager guarantees that the actual resource data will be loaded once, and it can be shared.
    ///
    /// ## Resource state
    ///
    /// Keep in mind, that the resource itself is a small state machine. It could be in these states:
    ///
    /// - [`ResourceState::Unloaded`] - a resource that has not started loading. Calling this method
    ///   updates the resource into the `Pending` state.
    /// - [`ResourceState::Pending`] - a resource is in the queue to load or still loading.
    /// - [`ResourceState::LoadError`] - a resource is failed to load.
    /// - [`ResourceState::Ok`] - a resource is successfully loaded.
    ///
    /// Actual resource state can be fetched by [`Resource::state`] method. If you know for sure that the resource
    /// is already loaded, then you can use [`Resource::data_ref`] to obtain a reference to the actual resource data.
    /// Keep in mind, that this method will panic if the resource non in `Ok` state.
    ///
    /// ## Panic
    ///
    /// This method will panic, if type UUID of `T` does not match the actual type UUID of the resource. If this
    /// is undesirable, use [`Self::try_request`] instead.
    pub fn request_resource<T>(&self, resource: &mut Resource<T>)
    where
        T: TypedResourceData,
    {
        let mut state = self.state();

        state.request_resource(&mut resource.untyped);

        if let Some(type_uuid) = resource.untyped.type_uuid_non_blocking() {
            let needed_type_uuid = <T as TypeUuidProvider>::type_uuid();
            if type_uuid != needed_type_uuid {
                panic!(
                    "Unable to get a resource of type {needed_type_uuid} from resource UUID {}! The resource is \
                    loaded but its actual data has type {type_uuid}!",
                    resource.resource_uuid(),
                );
            }
        }
    }

    /// Add the given resource to the resource manager, based on the resource's UUID,
    /// without initiating the loading of the resource. The given resource is modified
    /// to be a reference to the shared data of an existing resource with the same UUID.
    pub fn add_resource<T>(&self, resource: &mut Resource<T>)
    where
        T: TypedResourceData,
    {
        let mut state = self.state();

        state.add_resource(&mut resource.untyped);

        if let Some(type_uuid) = resource.untyped.type_uuid_non_blocking() {
            let needed_type_uuid = <T as TypeUuidProvider>::type_uuid();
            if type_uuid != needed_type_uuid {
                panic!(
                    "Unable to add a resource of type {needed_type_uuid} from resource UUID {}! The resource is \
                    loaded but its actual data has type {type_uuid}!",
                    resource.resource_uuid(),
                );
            }
        }
    }

    /// The same as [`Self::request`], but returns [`None`] if type UUID of `T` does not match the actual type UUID
    /// of the resource.
    ///
    /// ## Panic
    ///
    /// This method does not panic.
    pub fn try_request<T>(&self, path: impl AsRef<Path>) -> Option<Resource<T>>
    where
        T: TypedResourceData,
    {
        let mut state = self.state();
        let untyped = state.request(path.as_ref());
        if untyped
            .type_uuid_non_blocking()
            .is_some_and(|uuid| uuid == <T as TypeUuidProvider>::type_uuid())
            || state
                .loaders
                .safe_lock()
                .is_extension_matches_type::<T>(path.as_ref())
        {
            Some(Resource {
                untyped,
                phantom: PhantomData::<T>,
            })
        } else {
            None
        }
    }

    /// Tries to fetch a path of the given untyped resource. The path may be missing in a few cases:
    ///
    /// 1) The resource is in invalid state (not in [`ResourceState::Ok`]).
    /// 2) The resource wasn't registered in the resource registry.
    /// 3) The resource registry wasn't loaded.
    pub fn resource_path(&self, resource: impl AsRef<UntypedResource>) -> Option<PathBuf> {
        self.state().resource_path(resource.as_ref())
    }

    /// Tries to fetch a resource path associated with the given UUID. Returns [`None`] if there's
    /// no resource with the given UUID.
    pub fn uuid_to_resource_path(&self, resource_uuid: Uuid) -> Option<PathBuf> {
        self.state().uuid_to_resource_path(resource_uuid)
    }

    /// Same as [`Self::request`], but returns untyped resource.
    pub fn request_untyped<P>(&self, path: P) -> UntypedResource
    where
        P: AsRef<Path>,
    {
        self.state().request(path)
    }

    /// Tries to update the registry if possible on the current platform, and if not - try to load
    /// an existing one. Some platforms do not have a file system, so the registry must be prepared
    /// on a platform that **does** have it and then saved to be loaded later on. For example,
    /// WebAssembly platform does not have a file system and the resource manager will try to load
    /// an existing registry instead of updating it.
    pub fn update_or_load_registry(&self) {
        self.state().update_or_load_registry();
    }

    /// Adds a new resource loader of the given type.
    pub fn add_loader<T: ResourceLoader>(&self, loader: T) -> Option<T> {
        self.state().add_loader(loader)
    }

    /// Add the given resource to the manager and registers the resource as an external resource with the given
    /// path, updating the metadata file with the resource's UUID and updating the registry file with the resource's path.
    /// Calling this should only be necessary after newly creating the file in the given path by saving the resource
    /// to the file, otherwise the resource's path should already have been discovered by
    /// [`ResourceManager::update_or_load_registry`].
    /// If the manager already has a resource with this resource's UUID, return [`ResourceRegistrationError::AlreadyRegistered`].
    pub fn register(
        &self,
        resource: UntypedResource,
        path: impl AsRef<Path>,
    ) -> Result<(), ResourceRegistrationError> {
        self.state().register(resource, path)
    }

    /// Checks whether the given resource is a built-in resource instance or not.
    pub fn is_built_in_resource(&self, resource: impl AsRef<UntypedResource>) -> bool {
        self.state()
            .built_in_resources
            .is_built_in_resource(resource)
    }

    /// Creates a resource movement context.
    #[allow(clippy::await_holding_lock)]
    pub async fn make_resource_move_context(
        &self,
        src_path: impl AsRef<Path>,
        dest_path: impl AsRef<Path>,
        overwrite_existing: bool,
    ) -> Result<ResourceMoveContext, ResourceMovementError> {
        self.state()
            .make_resource_move_context(src_path, dest_path, overwrite_existing)
            .await
    }

    /// Returns `true` if a resource at the `src_path` can be moved to the `dest_path`, false -
    /// otherwise. Source path must be a valid resource path, and the dest path must have a valid
    /// new directory part of the path.
    #[allow(clippy::await_holding_lock)]
    pub async fn can_resource_be_moved(
        &self,
        src_path: impl AsRef<Path>,
        dest_path: impl AsRef<Path>,
        overwrite_existing: bool,
    ) -> bool {
        self.state()
            .can_resource_be_moved(src_path, dest_path, overwrite_existing)
            .await
    }

    /// Tries to move a resource at the given path to the new path. The path of the resource must be
    /// registered in the resource registry for the resource to be moveable. This method can also be
    /// used to rename the source file of a resource.
    #[allow(clippy::await_holding_lock)]
    pub async fn move_resource_by_path(
        &self,
        src_path: impl AsRef<Path>,
        dest_path: impl AsRef<Path>,
        overwrite_existing: bool,
    ) -> Result<(), ResourceMovementError> {
        self.state()
            .move_resource_by_path(src_path, dest_path, overwrite_existing)
            .await
    }

    /// Attempts to move a resource from its current location to the new path. The resource must
    /// be registered in the resource registry to be moveable. This method can also be used to
    /// rename the source file of a resource.
    pub async fn move_resource(
        &self,
        resource: impl AsRef<UntypedResource>,
        new_path: impl AsRef<Path>,
        overwrite_existing: bool,
    ) -> Result<(), ResourceMovementError> {
        let resource_path = self.resource_path(resource).ok_or_else(|| {
            FileError::Custom(
                "Cannot move the resource because it does not have a path!".to_string(),
            )
        })?;

        self.move_resource_by_path(resource_path, new_path, overwrite_existing)
            .await
    }

    /// Reloads all loaded resources. Normally it should never be called, because it is **very** heavy
    /// method! This method is asynchronous, it uses all available CPU power to reload resources as
    /// fast as possible.
    pub async fn reload_resources(&self) {
        let resources = self.state().reload_resources();
        join_all(resources).await;
    }

    /// Checks if there's a loader for the given resource path.
    pub fn is_supported_resource(&self, path: &Path) -> bool {
        self.state().is_supported_resource(path)
    }

    /// Checks if the given path is located inside the folder tracked by the resource registry.
    pub fn is_path_in_registry(&self, path: &Path) -> bool {
        self.state().is_path_in_registry(path)
    }

    /// Tries to move a folder to some other folder.
    #[allow(clippy::await_holding_lock)]
    pub async fn try_move_folder(
        &self,
        src_dir: &Path,
        dest_dir: &Path,
        overwrite_existing: bool,
    ) -> Result<(), FolderMovementError> {
        self.state()
            .try_move_folder(src_dir, dest_dir, overwrite_existing)
            .await
    }
}

impl ResourceManagerState {
    pub(crate) fn new(io: Arc<dyn ResourceIo>, task_pool: Arc<TaskPool>) -> Self {
        Self {
            resources: Default::default(),
            loaders: Default::default(),
            event_broadcaster: Default::default(),
            constructors_container: Default::default(),
            watcher: None,
            built_in_resources: Default::default(),
            resource_registry: Arc::new(Mutex::new(ResourceRegistry::new(io.clone()))),
            task_pool,
            resource_io: io,
        }
    }

    /// Tries to update the registry if possible on the current platform, and if not - try to load
    /// an existing one. Some platforms do not have a file system, so the registry must be prepared
    /// on a platform that **does** have it and then saved to be loaded later on. For example,
    /// WebAssembly platform does not have a file system and the resource manager will try to load
    /// an existing registry instead of updating it.
    pub fn update_or_load_registry(&self) {
        let resource_io = self.resource_io.clone();
        let resource_registry = self.resource_registry.clone();
        #[allow(unused_variables)]
        let excluded_folders = resource_registry.safe_lock().excluded_folders.clone();
        let registry_status = resource_registry.safe_lock().status_flag();
        registry_status.mark_as_loading();
        #[allow(unused_variables)]
        let task_loaders = self.loaders.clone();
        let path = resource_registry.safe_lock().path().to_path_buf();

        // Try to update the registry first.
        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        fyrox_core::futures::executor::block_on(async move {
            let new_data =
                ResourceRegistry::scan(resource_io.clone(), task_loaders, &path, excluded_folders)
                    .await;
            let mut registry_lock = resource_registry.safe_lock();
            registry_lock.modify().set_container(new_data);
            registry_status.mark_as_loaded();
        });

        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        self.task_pool.spawn_task(async move {
            use crate::registry::RegistryContainerExt;
            // Then load the registry.
            info!("Trying to load or update the registry at {path:?}...");
            match crate::registry::RegistryContainer::load_from_file(&path, &*resource_io).await {
                Ok(registry) => {
                    let mut registry_lock = resource_registry.safe_lock();
                    registry_lock.modify().set_container(registry);
                    info!("Resource registry was loaded from {path:?} successfully!");
                }
                Err(error) => {
                    err!("Unable to load resource registry! Reason: {error}.");
                }
            };
            registry_status.mark_as_loaded();
        });
    }

    /// Returns the task pool used by this resource manager.
    pub fn task_pool(&self) -> Arc<TaskPool> {
        self.task_pool.clone()
    }

    /// Set the IO source that the resource manager should use when
    /// loading assets
    pub fn set_resource_io(&mut self, resource_io: Arc<dyn ResourceIo>) {
        self.resource_io = resource_io;
    }

    /// Sets resource watcher which will track any modifications in file system and forcing
    /// the manager to reload changed resources. By default there is no watcher, since it
    /// may be an undesired effect to reload resources at runtime. This is very useful thing
    /// for fast iterative development.
    pub fn set_watcher(&mut self, watcher: Option<FileSystemWatcher>) {
        self.watcher = watcher;
    }

    /// Returns total amount of registered resources.
    pub fn count_registered_resources(&self) -> usize {
        self.resources.len()
    }

    /// Returns percentage of loading progress. This method is useful to show progress on
    /// loading screen in your game. This method could be used alone if your game depends
    /// only on external resources, or if your game doing some heavy calculations this value
    /// can be combined with progress of your tasks.
    pub fn loading_progress(&self) -> usize {
        let registered = self.count_registered_resources();
        if registered > 0 {
            self.count_loaded_resources() * 100 / registered
        } else {
            100
        }
    }

    fn try_get_event(&self) -> Option<Event> {
        self.watcher.as_ref()?.try_get_event()
    }

    /// Handle events in the file system relating to adding, removing, or modifying resources.
    /// This may involve updating the registry to reflect changes to the resources, and it may
    /// involve creating new meta files for resources that are missing meta files.
    pub fn process_filesystem_events(&mut self) {
        let mut modified_files = FxHashSet::default();
        while let Some(mut evt) = self.try_get_event() {
            if evt.need_rescan() {
                info!("Filesystem watcher has forced a rescan!");
                self.update_or_load_registry();
                self.reload_resources();
            } else {
                use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};
                use notify::EventKind;
                match evt.kind {
                    EventKind::Create(CreateKind::Any | CreateKind::File) => {
                        self.on_create_event(evt.paths.first())
                    }
                    EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                        self.on_remove_event(evt.paths.first())
                    }
                    EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                        self.on_create_event(evt.paths.first())
                    }
                    EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                        self.on_remove_event(evt.paths.first());
                        self.on_create_event(evt.paths.get(1));
                    }
                    EventKind::Modify(ModifyKind::Any | ModifyKind::Data(_)) => {
                        let path = evt.paths.get_mut(0).map(std::mem::take);
                        modified_files.insert(path);
                    }
                    EventKind::Remove(RemoveKind::Any | RemoveKind::File) => {
                        self.on_remove_event(evt.paths.first())
                    }
                    _ => (),
                }
            }
        }
        for path in modified_files {
            self.on_file_content_event(path.as_ref())
        }
    }

    fn on_create_event(&mut self, path: Option<&PathBuf>) {
        let path = some_or_return!(path);
        let mut relative_path = ok_or_return!(fyrox_core::make_relative_path(path));
        let ext = some_or_return!(relative_path.extension());
        let mut registry = self.resource_registry.safe_lock();
        if registry
            .excluded_folders
            .iter()
            .any(|folder| relative_path.starts_with(folder))
        {
            return;
        }
        // An event for a created file does not guarantee that the file actually exists.
        // It might have been deleted or renamed between being created and us receiving the event.
        if !block_on(self.resource_io.exists(&relative_path)) {
            return;
        }
        if ext == ResourceMetadata::EXTENSION {
            // Remove the meta extension from the path to get the path of the resource.
            relative_path.set_extension("");
            match registry.modify().read_metadata(relative_path.clone()) {
                Ok(RegistryUpdate {
                    changed,
                    value: metadata,
                }) => {
                    if changed {
                        info!(
                        "The resource {relative_path:?} was registered successfully with {} id from a newly created meta file!",
                        metadata.resource_id
                        )
                    }
                }
                Err(err) => {
                    err!(
                        "Unable to read the metadata for resource {relative_path:?}. Reason: {err}",
                    )
                }
            }
        } else if self
            .loaders
            .safe_lock()
            .is_supported_resource(&relative_path)
        {
            Self::on_new_resource_file(registry.modify(), relative_path.clone());
            drop(registry);
            if self.try_reload_resource_from_path(&relative_path) {
                info!(
                    "File {relative_path:?} was created, trying to reload a respective resource...",
                );
            }
        }
    }
    /// A new resource file has been discovered through an event. Check if there is a corresponding meta file,
    /// and if not then create the meta file with a randomly generated UUID.
    fn on_new_resource_file(mut registry: ResourceRegistryRefMut<'_>, relative_path: PathBuf) {
        match registry.read_metadata(relative_path.clone()) {
            Ok(RegistryUpdate {
                changed,
                value: metadata,
            }) => {
                if changed {
                    info!(
                    "The newly created resource {relative_path:?} was registered successfully with {} id from the meta file!",
                    metadata.resource_id
                    )
                }
            }
            Err(_) => {
                let uuid = Uuid::new_v4();
                match registry.write_metadata(uuid, relative_path.clone()) {
                    Ok(RegistryUpdate {
                        changed,
                        value: old_path,
                    }) => {
                        assert!(old_path.is_none());
                        if changed {
                            info!("The newly created resource {relative_path:?} was registered successfully with new id: {uuid}");
                        }
                    }
                    Err(err) => {
                        err!("Unable to register the resource {relative_path:?}. Reason: {err}")
                    }
                }
            }
        }
    }
    fn on_remove_event(&mut self, path: Option<&PathBuf>) {
        let path = some_or_return!(path);
        let mut relative_path = ok_or_return!(fyrox_core::make_relative_path(path));
        let ext = some_or_return!(relative_path.extension());
        // For the purposes of updating the registry, only the removal of meta files is relevant.
        if ext != ResourceMetadata::EXTENSION {
            return;
        }
        // An event for a deleted file does not guarantee that the file does not exist.
        // It might have been created again between being deleted and us receiving the event.
        // If the file exists now, ignore the remove event.
        if block_on(self.resource_io.exists(&relative_path)) {
            return;
        }

        let mut registry = self.resource_registry.safe_lock();
        if registry
            .excluded_folders
            .iter()
            .any(|folder| relative_path.starts_with(folder))
        {
            return;
        }
        // Remove the meta extension from the path to get the path of the resource.
        relative_path.set_extension("");
        // Check whether the resource file exists, and if not then there is nothing more to do.
        if !block_on(self.resource_io.exists(&relative_path)) {
            return;
        }
        // Recreate the meta file, using the current UUID for the resource's path, if possible.
        let uuid = match registry.path_to_uuid(&relative_path) {
            Some(uuid) => {
                info!("The meta file for {relative_path:?} was removed, but its UUID is still in memory: {uuid}");
                uuid
            }
            None => {
                info!("The meta file for {relative_path:?} was removed and its UUID is lost!");
                Uuid::new_v4()
            }
        };
        let result = registry
            .modify()
            .write_metadata(uuid, relative_path.clone());
        match result {
            Ok(RegistryUpdate { changed, .. }) => {
                if changed {
                    info!(
                        "The resource {relative_path:?} was registered successfully with {uuid} id after its meta file was removed!",
                    )
                } else {
                    info!(
                        "The meta file for resource {relative_path:?} was recreated with {uuid} id!",
                    )
                }
            }
            Err(err) => {
                err!(
                    "Unable to register the resource {relative_path:?} after its meta file was removed. Reason: {err}",
                )
            }
        }
    }
    fn on_file_content_event(&mut self, path: Option<&PathBuf>) {
        let path = some_or_return!(path);
        let mut relative_path = ok_or_return!(fyrox_core::make_relative_path(path));
        let ext = some_or_return!(relative_path.extension());
        let mut registry = self.resource_registry.safe_lock();
        if registry
            .excluded_folders
            .iter()
            .any(|folder| relative_path.starts_with(folder))
        {
            return;
        }
        // An event for a modified file does not guarantee that the file actually exists.
        // It might have been deleted or renamed between being modified and us receiving the event.
        if !block_on(self.resource_io.exists(&relative_path)) {
            return;
        }

        if ext == ResourceMetadata::EXTENSION {
            // Remove the meta extension from the path to get the path of the resource.
            relative_path.set_extension("");
            match registry.modify().read_metadata(relative_path.clone()) {
                Ok(RegistryUpdate {
                    changed,
                    value: metadata,
                }) => {
                    if changed {
                        info!(
                            "The resource {relative_path:?} was registered successfully with {} id after meta file modification",
                            metadata.resource_id
                        )
                    }
                }
                Err(err) => {
                    err!(
                        "Unable to read the metadata for resource {relative_path:?} after meta file modification. Reason: {err}",
                    )
                }
            }
        } else if self
            .loaders
            .safe_lock()
            .is_supported_resource(&relative_path)
        {
            drop(registry);
            if self.try_reload_resource_from_path(&relative_path) {
                info!(
                    "File {relative_path:?} was changed, trying to reload a respective resource...",
                );
            }
        }
    }

    /// Update resource containers and do hot-reloading.
    ///
    /// Resources are removed if they're not used
    /// or reloaded if they have changed in disk.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn update(&mut self, dt: f32) {
        self.resources.retain_mut(|resource| {
            // One usage means that the resource has single owner, and that owner
            // is this container. Such resources have limited life time, if the time
            // runs out before it gets shared again, the resource will be deleted.
            if resource.value.use_count() <= 1 {
                resource.time_to_live -= dt;
                if resource.time_to_live <= 0.0 {
                    let registry = self.resource_registry.safe_lock();
                    let resource_uuid = resource.resource_uuid();
                    if let Some(path) = registry.uuid_to_path(resource_uuid) {
                        info!("Resource {path:?} destroyed because it is not used anymore!",);
                        self.event_broadcaster
                            .broadcast(ResourceEvent::Removed(path.to_path_buf()));
                    }

                    false
                } else {
                    // Keep resource alive for short period of time.
                    true
                }
            } else {
                // Make sure to reset timer if a resource is used by more than one owner.
                resource.time_to_live = DEFAULT_RESOURCE_LIFETIME;

                // Keep resource alive while it has more than one owner.
                true
            }
        });
    }

    fn add_resource_and_notify(&mut self, resource: UntypedResource) {
        self.event_broadcaster
            .broadcast(ResourceEvent::Added(resource.clone()));

        self.resources.push(TimedEntry {
            value: resource,
            time_to_live: DEFAULT_RESOURCE_LIFETIME,
        });
    }

    /// Tries to find a resource by its path. Returns None if no resource was found.
    ///
    /// # Complexity
    ///
    /// O(n)
    pub fn find_by_uuid(&self, uuid: Uuid) -> Option<&UntypedResource> {
        self.resources
            .iter()
            .find(|entry| entry.value.resource_uuid() == uuid)
            .map(|entry| &entry.value)
    }

    /// Tries to find a resource by a path. Returns None if no resource was found.
    ///
    /// # Complexity
    ///
    /// O(n)
    pub fn find_by_path(&self, path: &Path) -> Option<&UntypedResource> {
        let registry = self.resource_registry.safe_lock();
        self.resources.iter().find_map(|entry| {
            if registry.uuid_to_path(entry.resource_uuid()) == Some(path) {
                return Some(&entry.value);
            }
            None
        })
    }

    /// Returns total amount of resources in the container.
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Returns true if the resource manager has no resources.
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Creates an iterator over resources in the manager.
    pub fn iter(&self) -> impl Iterator<Item = &UntypedResource> {
        self.resources.iter().map(|entry| &entry.value)
    }

    /// Immediately destroys all resources in the manager that are not used anywhere else.
    pub fn destroy_unused_resources(&mut self) {
        self.resources
            .retain(|resource| resource.value.use_count() > 1);
    }

    /// Returns total amount of resources that still loading.
    pub fn count_pending_resources(&self) -> usize {
        self.resources.iter().filter(|r| r.is_loading()).count()
    }

    /// Returns total amount of completely loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        self.resources.iter().filter(|r| r.is_ok()).count()
    }

    /// Returns a set of resource handled by this container.
    pub fn resources(&self) -> Vec<UntypedResource> {
        self.resources.iter().map(|t| t.value.clone()).collect()
    }

    /// Registers a new built-in resource, so it becomes accessible via [`Self::request`].
    pub fn register_built_in_resource<T: TypedResourceData>(
        &mut self,
        resource: BuiltInResource<T>,
    ) -> Option<UntypedBuiltInResource> {
        self.built_in_resources.add(resource)
    }

    /// Find the resource for the given UUID without loading the resource.
    /// Searches the resource manager to find a resource with the given UUID, including built-in resources.
    /// If no resource is found, a new unloaded external resource is returned for the given UUID,
    /// because it is presumed that this is a real UUID for some resource that is not currently managed
    /// and therefore it should be added to the manager.
    pub fn find_uuid(&mut self, uuid: Uuid) -> UntypedResource {
        if let Some(built_in_resource) = self.built_in_resources.find_by_uuid(uuid) {
            return built_in_resource.resource.clone();
        }

        if let Some(existing) = self.find_by_uuid(uuid) {
            existing.clone()
        } else {
            let resource = UntypedResource::new_unloaded(uuid);
            self.add_resource_and_notify(resource.clone());
            resource
        }
    }

    /// Searches the resource manager and the registry to find a resource with the given path,
    /// including built-in resources. If no resource is found, a new UUID is generated and the
    /// path is added to the registry and an unloaded resource is returned.
    pub fn find<P>(&mut self, path: P) -> UntypedResource
    where
        P: AsRef<Path>,
    {
        if let Some(built_in_resource) = self.built_in_resources.get(path.as_ref()) {
            return built_in_resource.resource.clone();
        }

        let path = ResourceRegistry::normalize_path(path);

        if let Some(existing) = self.find_by_resource_path(&path) {
            existing.clone()
        } else {
            let mut registry = self.resource_registry.safe_lock();
            let uuid = if let Some(uuid) = registry.path_to_uuid(&path) {
                uuid
            } else {
                let uuid = Uuid::new_v4();
                registry.modify().register(uuid, path);
                uuid
            };
            drop(registry);
            let resource = UntypedResource::new_unloaded(uuid);
            self.add_resource_and_notify(resource.clone());
            resource
        }
    }

    /// Tries to load the resource at the given path.
    pub fn request<P>(&mut self, path: P) -> UntypedResource
    where
        P: AsRef<Path>,
    {
        if let Some(built_in_resource) = self.built_in_resources.get(path.as_ref()) {
            return built_in_resource.resource.clone();
        }

        let path = ResourceRegistry::normalize_path(path);

        self.find_or_load(path)
    }

    /// Tries to load the resource for the given UUID.
    pub fn request_uuid(&mut self, uuid: Uuid) -> UntypedResource {
        let mut resource = uuid.into();
        self.request_resource(&mut resource);
        resource
    }

    /// Use the registry to find a resource with the given path, blocking until the registry is loaded if necessary.
    fn find_by_resource_path(&self, path_to_search: &Path) -> Option<&UntypedResource> {
        let registry = self.resource_registry.safe_lock();
        self.resources
            .iter()
            .find(move |entry| registry.uuid_to_path(entry.resource_uuid()) == Some(path_to_search))
            .map(|entry| &entry.value)
    }

    /// If a resource exists for the given path, return it.
    /// Otherwise, check the registry for a UUID for the path, create
    /// a resource, begin loading, and return the resource.
    /// If the given path does not correspond to any registered UUID,
    /// create and return an error resource.
    fn find_or_load(&mut self, path: PathBuf) -> UntypedResource {
        match self.find_by_resource_path(&path) {
            Some(existing) => existing.clone(),
            None => self.load_resource(path),
        }
    }

    fn load_resource(&mut self, path: PathBuf) -> UntypedResource {
        let mut registry = self.resource_registry.safe_lock();
        let uuid = if let Some(uuid) = registry.path_to_uuid(&path) {
            uuid
        } else {
            let uuid = Uuid::new_v4();
            registry.modify().register(uuid, path);
            uuid
        };
        drop(registry);
        let resource = UntypedResource::new_pending(uuid, ResourceKind::External);
        self.add_resource_and_notify(resource.clone());
        self.spawn_loading_task(resource.clone(), false);
        resource
    }

    /// Add a task to the task pool to load the given resource.
    /// Panic if the given resource is unregistered or embedded.
    fn spawn_loading_task(&self, mut resource: UntypedResource, reload: bool) {
        let event_broadcaster = self.event_broadcaster.clone();
        let loaders = self.loaders.clone();
        let registry = self.resource_registry.clone();
        let io = self.resource_io.clone();
        let registry_status = registry.safe_lock().status_flag();

        self.task_pool.spawn_task(async move {
            // Wait until the registry is fully loaded.
            let registry_status = registry_status.await;
            if registry_status == ResourceRegistryStatus::Unknown {
                resource.commit_error(
                    PathBuf::default(),
                    LoadError::new("The resource registry is unavailable!".to_string()),
                );
                return;
            }

            let Some(path) = registry
                .safe_lock()
                .uuid_to_path(resource.resource_uuid())
                .map(|p| p.to_path_buf())
            else {
                let error = format!(
                    "Resource {} failed to load. The path was not found \
                        in the registry!",
                    resource.resource_uuid(),
                );

                resource.commit_error(PathBuf::default(), error);
                return;
            };

            // Try to find a loader for the resource.
            let loader_future = loaders
                .safe_lock()
                .loader_for(&path)
                .map(|loader| loader.load(path.clone(), io));

            if let Some(loader_future) = loader_future {
                match loader_future.await {
                    Ok(data) => {
                        let data = data.0;

                        let mut header = resource.lock();

                        assert!(header.kind.is_external());

                        header.state.commit(ResourceState::Ok {
                            data: ResourceDataWrapper(data),
                        });

                        drop(header);

                        event_broadcaster.broadcast_loaded_or_reloaded(resource, reload);

                        Log::info(format!(
                            "Resource {} was loaded successfully!",
                            path.display()
                        ));
                    }
                    Err(error) => {
                        if reload {
                            if resource.is_ok() {
                                info!("Resource {path:?} failed to reload, keeping the existing version. Reason: {error}");
                            }
                            else
                            {
                                info!("Resource {path:?} failed to reload. Reason: {error}");
                                resource.commit_error(path.to_path_buf(), error);
                            }
                        }
                        else
                        {
                            info!("Resource {path:?} failed to load. Reason: {error}");
                            resource.commit_error(path.to_path_buf(), error);
                        }
                    }
                }
            } else {
                let error = format!("There's no resource loader for {path:?} resource!",);
                resource.commit_error(path, error);
            }
        });
    }

    /// Tries to fetch a path of the given untyped resource. The path may be missing in a few cases:
    ///
    /// 1) The resource wasn't registered in the resource registry.
    /// 2) The resource registry wasn't loaded.
    ///
    /// ## Built-in resources
    ///
    /// As a last resort, this method tries to find a built-in resource descriptor corresponding
    /// to the given resource and returns its "path". In reality, it is just a string id, since
    /// built-in resources are stored inside the binary.
    pub fn resource_path(&self, resource: &UntypedResource) -> Option<PathBuf> {
        self.uuid_to_resource_path(resource.resource_uuid())
    }

    /// Tries to fetch a resource path associated with the given UUID. Returns [`None`] if there's
    /// no resource with the given UUID.
    ///
    /// ## Built-in resources
    ///
    /// As a last resort, this method tries to find a built-in resource descriptor corresponding
    /// to the given resource uuid and returns its "path". In reality, it is just a string id, since
    /// built-in resources are stored inside the binary.
    pub fn uuid_to_resource_path(&self, resource_uuid: Uuid) -> Option<PathBuf> {
        if let Some(path) = self
            .resource_registry
            .safe_lock()
            .uuid_to_path_buf(resource_uuid)
        {
            Some(path)
        } else {
            self.built_in_resources
                .find_by_uuid(resource_uuid)
                .map(|built_in_resource| built_in_resource.id.clone())
        }
    }

    /// Adds a new resource loader of the given type.
    pub fn add_loader<T: ResourceLoader>(&self, loader: T) -> Option<T> {
        self.loaders.safe_lock().set(loader)
    }

    /// Tries to load the given resource, based on the resource's UUID,
    /// and adds the resource to the manager if it is not already in the manager.
    pub fn request_resource(&mut self, resource: &mut UntypedResource) {
        if let Some(r) = self.find_by_uuid(resource.resource_uuid()) {
            // We are already managing a resource with this UUID, so modify the given
            // resource to point to our resource.
            *resource = r.clone();
            if resource.is_unloaded() {
                // If the resource is in the unloaded state, start it loading, because it has been requested.
                resource.make_pending();
                self.spawn_loading_task(resource.clone(), false);
            }
        } else if let Some(r) = self
            .built_in_resources
            .find_by_uuid(resource.resource_uuid())
        {
            // A built-in resource has this UUID, so modify the given resource to
            // point to the built-in resource.
            *resource = r.resource.clone();
        } else if resource.is_ok() || resource.is_embedded() {
            // This is an unknown resource, but it is ready to use, so add it to our managed
            // resources.
            self.add_resource_and_notify(resource.clone());
        } else {
            // This is an unknown resource and it is not ready to use, so add it to our managed
            // resources and begin the loading process.
            resource.make_pending();
            self.add_resource_and_notify(resource.clone());
            self.spawn_loading_task(resource.clone(), false);
        }
    }

    /// Add the given resource to the resource manager, based on the resource's UUID,
    /// without initiating the loading of the resource. The given resource may be modified
    /// to be a reference to the shared data of an existing resource with the same UUID.
    pub fn add_resource(&mut self, resource: &mut UntypedResource) {
        if let Some(r) = self.find_by_uuid(resource.resource_uuid()) {
            // We are already managing a resource with this UUID, so modify the given
            // resource to point to our resource.
            *resource = r.clone();
        } else if let Some(r) = self
            .built_in_resources
            .find_by_uuid(resource.resource_uuid())
        {
            // A built-in resource has this UUID, so modify the given resource to
            // point to the built in resource.
            *resource = r.resource.clone();
        } else {
            // This is an unknown resource, so add it to our managed resources.
            self.add_resource_and_notify(resource.clone());
        }
    }

    /// Add the given resource to the manager and registers the resource as an external resource with the given
    /// path, updating the metadata file with the resource's UUID and updating the registry file with the resource's path.
    /// Calling this should only be necessary after newly creating the file in the given path by saving the resource
    /// to the file, otherwise the resource's path should already have been discovered by
    /// [`ResourceManagerState::update_or_load_registry`].
    /// If the manager already has a resource with this resource's UUID, return [`ResourceRegistrationError::AlreadyRegistered`].
    pub fn register(
        &mut self,
        resource: UntypedResource,
        path: impl AsRef<Path>,
    ) -> Result<(), ResourceRegistrationError> {
        let resource_uuid = resource.resource_uuid();
        if self.find_by_uuid(resource_uuid).is_some() {
            return Err(ResourceRegistrationError::AlreadyRegistered);
        }
        let path = ResourceRegistry::normalize_path(path);
        {
            let mut header = resource.lock();
            header.kind.make_external();
            let mut registry = self.resource_registry.safe_lock();
            let mut ctx = registry.modify();
            if ctx.write_metadata(resource_uuid, path).is_err() {
                return Err(ResourceRegistrationError::UnableToCreateMetadata);
            }
        }
        self.add_resource_and_notify(resource);
        Ok(())
    }

    /// Reloads a single resource. Does nothing in case of built-in resources.
    /// Log an error if the resource cannot be reloaded.
    pub fn reload_resource(&mut self, resource: UntypedResource) {
        if self.built_in_resources.is_built_in_resource(&resource) {
            return;
        }
        let mut header = resource.lock();
        if !header.state.is_loading() {
            if !header.state.is_ok() {
                header.state.switch_to_pending_state();
            }
            drop(header);
            self.spawn_loading_task(resource, true)
        }
    }

    /// Reloads all resources in the container. Returns a list of resources that will be reloaded.
    /// You can use the list to wait until all resources are loading.
    pub fn reload_resources(&mut self) -> Vec<UntypedResource> {
        let resources = self
            .resources
            .iter()
            .map(|r| r.value.clone())
            .collect::<Vec<_>>();

        for resource in resources.iter().cloned() {
            self.reload_resource(resource);
        }

        resources
    }

    /// Wait until all resources are loaded (or failed to load).
    pub fn get_wait_context(&self) -> ResourceWaitContext {
        ResourceWaitContext {
            resources: self
                .resources
                .iter()
                .map(|e| e.value.clone())
                .collect::<Vec<_>>(),
        }
    }

    /// Tries to reload a resource at the given path, and returns true if a reload will
    /// actually begin. Returns false if the resource was already loading or
    /// cannot be reloaded.
    pub fn try_reload_resource_from_path(&mut self, path: &Path) -> bool {
        // Do not try to reload unsupported resources.
        if !self.loaders.safe_lock().is_supported_resource(path) {
            return false;
        }

        let Some(resource) = self.find_by_resource_path(path) else {
            return false;
        };
        let header = resource.lock();
        if header.state.is_loading() {
            return false;
        }
        drop(header);
        self.reload_resource(resource.clone());
        true
    }

    /// Creates a resource movement context.
    #[allow(clippy::await_holding_lock)]
    pub async fn make_resource_move_context(
        &self,
        src_path: impl AsRef<Path>,
        dest_path: impl AsRef<Path>,
        overwrite_existing: bool,
    ) -> Result<ResourceMoveContext, ResourceMovementError> {
        let src_path = src_path.as_ref();
        let dest_path = dest_path.as_ref();

        let relative_src_path = fyrox_core::make_relative_path(src_path)?;
        let relative_dest_path = fyrox_core::make_relative_path(dest_path)?;

        if let Some(file_stem) = relative_dest_path.file_stem() {
            if !self.resource_io.is_valid_file_name(file_stem) {
                return Err(ResourceMovementError::DestinationPathIsInvalid {
                    src_path: relative_src_path.clone(),
                    dest_path: relative_dest_path.clone(),
                });
            }
        }

        if !overwrite_existing && self.resource_io.exists(&relative_dest_path).await {
            return Err(ResourceMovementError::AlreadyExist {
                src_path: relative_src_path.clone(),
                dest_path: relative_dest_path.clone(),
            });
        }

        let registry_lock_guard = self.resource_registry.safe_lock();
        let absolute_registry_dir = if let Some(directory) = registry_lock_guard.directory() {
            fyrox_core::replace_slashes(self.resource_io.canonicalize_path(directory).await?)
        } else {
            return Err(ResourceMovementError::ResourceRegistryLocationUnknown {
                resource_path: relative_src_path.clone(),
            });
        };
        let resource_uuid = registry_lock_guard
            .path_to_uuid(&relative_src_path)
            .ok_or_else(|| ResourceMovementError::NotInRegistry {
                resource_path: relative_src_path.clone(),
            })?;

        let relative_dest_dir = relative_dest_path.parent().unwrap_or(Path::new("."));
        let absolute_dest_dir = fyrox_core::replace_slashes(
            self.resource_io
                .canonicalize_path(relative_dest_dir)
                .await?,
        );

        let absolute_src_path = fyrox_core::replace_slashes(
            self.resource_io
                .canonicalize_path(&relative_src_path)
                .await?,
        );
        if !absolute_dest_dir.starts_with(&absolute_registry_dir) {
            return Err(ResourceMovementError::OutsideOfRegistry {
                absolute_src_path,
                absolute_dest_dir,
                absolute_registry_dir,
            });
        }

        drop(registry_lock_guard);

        Ok(ResourceMoveContext {
            relative_src_path,
            relative_dest_path,
            resource_uuid,
        })
    }

    /// Returns `true` if a resource at the `src_path` can be moved to the `dest_path`, false -
    /// otherwise. Source path must be a valid resource path, and the dest path must have a valid
    /// new directory part of the path.
    pub async fn can_resource_be_moved(
        &self,
        src_path: impl AsRef<Path>,
        dest_path: impl AsRef<Path>,
        overwrite_existing: bool,
    ) -> bool {
        self.make_resource_move_context(src_path, dest_path, overwrite_existing)
            .await
            .is_ok()
    }

    /// Tries to move a resource at the given path to the new path. The path of the resource must be
    /// registered in the resource registry for the resource to be moveable. This method can also be
    /// used to rename the source file of a resource.
    pub async fn move_resource_by_path(
        &self,
        src_path: impl AsRef<Path>,
        dest_path: impl AsRef<Path>,
        overwrite_existing: bool,
    ) -> Result<(), ResourceMovementError> {
        let ResourceMoveContext {
            relative_src_path,
            relative_dest_path,

            resource_uuid,
        } = self
            .make_resource_move_context(src_path, dest_path, overwrite_existing)
            .await?;

        // Move the file with its optional import options and mandatory metadata.
        self.resource_io
            .move_file(&relative_src_path, &relative_dest_path)
            .await?;

        let current_path = self
            .resource_registry
            .safe_lock()
            .modify()
            .register(resource_uuid, relative_dest_path.to_path_buf());
        assert_eq!(current_path.value.as_ref(), Some(&relative_src_path));

        let options_path = append_extension(&relative_src_path, OPTIONS_EXTENSION);
        if self.resource_io.exists(&options_path).await {
            let new_options_path = append_extension(&relative_dest_path, OPTIONS_EXTENSION);
            self.resource_io
                .move_file(&options_path, &new_options_path)
                .await?;
        }

        let metadata_path = append_extension(&relative_src_path, ResourceMetadata::EXTENSION);
        if self.resource_io.exists(&metadata_path).await {
            let new_metadata_path =
                append_extension(&relative_dest_path, ResourceMetadata::EXTENSION);
            self.resource_io
                .move_file(&metadata_path, &new_metadata_path)
                .await?;
        }

        Ok(())
    }

    /// Attempts to move a resource from its current location to the new path. The resource must
    /// be registered in the resource registry to be moveable. This method can also be used to
    /// rename the source file of a resource.
    pub async fn move_resource(
        &self,
        resource: impl AsRef<UntypedResource>,
        new_path: impl AsRef<Path>,
        overwrite_existing: bool,
    ) -> Result<(), ResourceMovementError> {
        let resource_path = self.resource_path(resource.as_ref()).ok_or_else(|| {
            FileError::Custom(
                "Cannot move the resource because it does not have a path!".to_string(),
            )
        })?;

        self.move_resource_by_path(resource_path, new_path, overwrite_existing)
            .await
    }

    /// Checks if there's a loader for the given resource path.
    pub fn is_supported_resource(&self, path: &Path) -> bool {
        let ext = some_or_return!(path.extension(), false);
        let ext = some_or_return!(ext.to_str(), false);

        self.loaders
            .safe_lock()
            .iter()
            .any(|loader| loader.supports_extension(ext))
    }

    /// Checks if the given path is located inside the folder tracked by the resource registry.
    pub fn is_path_in_registry(&self, path: &Path) -> bool {
        let registry = self.resource_registry.safe_lock();
        if let Some(registry_directory) = registry.directory() {
            if let Ok(canonical_registry_path) = registry_directory.canonicalize() {
                if let Ok(canonical_path) = path.canonicalize() {
                    return canonical_path.starts_with(canonical_registry_path);
                }
            }
        }
        false
    }

    /// Tries to move a folder to some other folder.
    pub async fn try_move_folder(
        &self,
        src_dir: &Path,
        dest_dir: &Path,
        overwrite_existing: bool,
    ) -> Result<(), FolderMovementError> {
        if dest_dir.starts_with(src_dir) {
            return Err(FolderMovementError::HierarchyError {
                src_dir: src_dir.to_path_buf(),
                dest_dir: dest_dir.to_path_buf(),
            });
        }

        // Early validation to prevent error spam when trying to move a folder out of the
        // assets directory.
        if !self.is_path_in_registry(dest_dir) {
            return Err(FolderMovementError::NotInRegistry {
                dest_dir: dest_dir.to_path_buf(),
            });
        }

        // At this point we have a folder dropped on some other folder. In this case
        // we need to move all the assets from the dropped folder to a new subfolder (with the same
        // name as the dropped folder) of the other folder first. After that we can move the rest
        // of the files and finally delete the dropped folder.
        let mut what_where_stack = vec![(src_dir.to_path_buf(), dest_dir.to_path_buf())];
        while let Some((src_dir, target_dir)) = what_where_stack.pop() {
            let src_dir_name = some_or_continue!(src_dir.file_name());

            let target_sub_dir = target_dir.join(src_dir_name);
            if !self.resource_io.exists(&target_sub_dir).await {
                std::fs::create_dir(&target_sub_dir)?;
            }

            let target_sub_dir_normalized = ok_or_continue!(make_relative_path(&target_sub_dir));

            for path in self.resource_io.walk_directory(&src_dir, 1).await? {
                if path.is_file() {
                    let file_name = some_or_continue!(path.file_name());
                    if self.is_supported_resource(&path) {
                        let dest_path = target_sub_dir_normalized.join(file_name);
                        self.move_resource_by_path(path, &dest_path, overwrite_existing)
                            .await?;
                    }
                } else if path.is_dir() && path != src_dir {
                    // Sub-folders will be processed after all assets from current dir
                    // were moved.
                    what_where_stack.push((path, target_sub_dir.clone()));
                }
            }
        }

        std::fs::remove_dir_all(src_dir)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::io::FsResourceIo;
    use crate::{
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
        ResourceData,
    };
    use fyrox_core::{
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::{Visit, VisitResult, Visitor},
        TypeUuidProvider,
    };
    use std::{error::Error, fs::File, time::Duration};

    #[derive(Debug, Default, Clone, Reflect, Visit)]
    struct Stub {}

    impl TypeUuidProvider for Stub {
        fn type_uuid() -> Uuid {
            uuid!("9d873ff4-3126-47e1-a492-7cd8e7168239")
        }
    }

    impl ResourceData for Stub {
        fn type_uuid(&self) -> Uuid {
            <Self as TypeUuidProvider>::type_uuid()
        }

        fn save(&mut self, _path: &Path) -> Result<(), Box<dyn Error>> {
            Err("Saving is not supported!".to_string().into())
        }

        fn can_be_saved(&self) -> bool {
            false
        }

        fn try_clone_box(&self) -> Option<Box<dyn ResourceData>> {
            Some(Box::new(self.clone()))
        }
    }

    impl ResourceLoader for Stub {
        fn extensions(&self) -> &[&str] {
            &["txt"]
        }

        fn data_type_uuid(&self) -> Uuid {
            <Stub as TypeUuidProvider>::type_uuid()
        }

        fn load(&self, _path: PathBuf, _io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
            Box::pin(async move { Ok(LoaderPayload::new(Stub::default())) })
        }
    }

    fn new_resource_manager() -> ResourceManagerState {
        ResourceManagerState::new(Arc::new(FsResourceIo), Arc::new(Default::default()))
    }

    fn remove_file_if_exists(path: &Path) -> std::io::Result<()> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) => match e.kind() {
                std::io::ErrorKind::NotFound => Ok(()),
                _ => Err(e),
            },
        }
    }

    #[test]
    fn resource_wait_context_is_all_loaded() {
        assert!(ResourceWaitContext::default().is_all_loaded());

        let cx = ResourceWaitContext {
            resources: vec![
                UntypedResource::new_pending(Default::default(), ResourceKind::External),
                UntypedResource::new_load_error(
                    ResourceKind::External,
                    Default::default(),
                    LoadError::default(),
                ),
            ],
        };
        assert!(!cx.is_all_loaded());
    }

    #[test]
    fn resource_manager_state_new() {
        let state = new_resource_manager();

        assert!(state.resources.is_empty());
        assert!(state.loaders.safe_lock().is_empty());
        assert!(state.built_in_resources.is_empty());
        assert!(state.constructors_container.is_empty());
        assert!(state.watcher.is_none());
        assert!(state.is_empty());
    }

    #[test]
    fn resource_manager_state_set_watcher() {
        let mut state = new_resource_manager();
        assert!(state.watcher.is_none());

        let path = PathBuf::from("test.txt");
        if File::create(path.clone()).is_ok() {
            let watcher = FileSystemWatcher::new(path.clone(), Duration::from_secs(1));
            state.set_watcher(watcher.ok());
            assert!(state.watcher.is_some());
        }
    }

    #[test]
    fn resource_manager_state_push() {
        std::fs::create_dir_all("data").expect("Could not create data directory.");
        let mut state = new_resource_manager();

        assert_eq!(state.count_loaded_resources(), 0);
        assert_eq!(state.count_pending_resources(), 0);
        assert_eq!(state.count_registered_resources(), 0);
        assert_eq!(state.len(), 0);

        assert!(state
            .register(
                UntypedResource::new_pending(Default::default(), ResourceKind::External),
                "foo.bar",
            )
            .is_ok());
        assert!(state
            .register(
                UntypedResource::new_load_error(
                    ResourceKind::External,
                    Default::default(),
                    LoadError::default()
                ),
                "foo.bar",
            )
            .is_ok());
        assert!(state
            .register(
                UntypedResource::new_ok(Uuid::new_v4(), Default::default(), Stub {}),
                "foo.bar",
            )
            .is_ok());

        assert_eq!(state.count_registered_resources(), 3);
        assert_eq!(state.len(), 3);
    }

    #[test]
    fn resource_manager_state_loading_progress() {
        let mut state = new_resource_manager();

        assert_eq!(state.loading_progress(), 100);

        state
            .register(
                UntypedResource::new_ok(Uuid::new_v4(), Default::default(), Stub {}),
                "foo.bar",
            )
            .unwrap();

        assert_eq!(state.loading_progress(), 100);
    }

    #[test]
    fn resource_manager_state_find() {
        let mut state = new_resource_manager();

        let path = Path::new("foo.txt");

        assert!(state.find_by_path(path).is_none());

        let resource = UntypedResource::new_ok(Uuid::new_v4(), Default::default(), Stub {});
        state.register(resource.clone(), path).unwrap();

        assert_eq!(state.find_by_path(path), Some(&resource));
    }

    #[test]
    fn resource_manager_state_resources() {
        let mut state = new_resource_manager();

        assert_eq!(state.resources(), Vec::new());

        let r1 = UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::External, Stub {});
        let r2 = UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::External, Stub {});
        let r3 = UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::External, Stub {});
        state.register(r1.clone(), "foo1.txt").unwrap();
        state.register(r2.clone(), "foo2.txt").unwrap();
        state.register(r3.clone(), "foo3.txt").unwrap();

        assert_eq!(state.resources(), vec![r1.clone(), r2.clone(), r3.clone()]);
        assert!(state.iter().eq([&r1, &r2, &r3]));
    }

    #[test]
    fn resource_manager_state_destroy_unused_resources() {
        let mut state = new_resource_manager();

        state
            .register(
                UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::External, Stub {}),
                "foo1.txt",
            )
            .unwrap();
        assert_eq!(state.len(), 1);

        state.destroy_unused_resources();
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn resource_manager_state_request() {
        let mut state = new_resource_manager();
        let path = PathBuf::from("test.txt");

        let resource = UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::External, Stub {});
        state.register(resource.clone(), &path).unwrap();

        let res = state.request(&path);
        assert_eq!(res, resource);

        let res = state.request(path);

        assert_eq!(res.kind(), ResourceKind::External);
        assert!(!res.is_loading());
    }

    #[test]
    fn resource_manager_state_get_wait_context() {
        let mut state = new_resource_manager();

        let resource = UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::External, Stub {});
        state.add_resource_and_notify(resource.clone());
        let cx = state.get_wait_context();

        assert!(cx.resources.eq(&vec![resource]));
    }

    #[test]
    fn resource_manager_new() {
        let manager = ResourceManager::new(Arc::new(FsResourceIo), Arc::new(Default::default()));

        assert!(manager.state.safe_lock().is_empty());
        assert!(manager.state().is_empty());
    }

    #[test]
    fn resource_manager_register() {
        std::fs::create_dir_all("data").expect("Could not create data directory.");
        let manager = ResourceManager::new(Arc::new(FsResourceIo), Arc::new(Default::default()));
        let path = PathBuf::from("test.txt");
        let metapath = append_extension(&path, ResourceMetadata::EXTENSION);
        remove_file_if_exists(&metapath).unwrap();

        let resource = UntypedResource::new_pending(Default::default(), ResourceKind::External);
        let res = manager.register(resource.clone(), path.clone());
        assert!(res.is_ok());

        let metadata = block_on(ResourceMetadata::load_from_file_async(
            &metapath,
            &*manager.resource_io(),
        ))
        .expect("Reading meta file failed");
        assert_eq!(resource.resource_uuid(), metadata.resource_id);

        let uuid = Uuid::new_v4();
        let resource = UntypedResource::new_ok(uuid, ResourceKind::External, Stub {});
        let res = manager.register(resource.clone(), path.clone());
        assert!(res.is_ok());

        assert_eq!(resource.resource_uuid(), uuid);
        let metadata = block_on(ResourceMetadata::load_from_file_async(
            &metapath,
            &*manager.resource_io(),
        ))
        .expect("Reading meta file failed");
        assert_eq!(metadata.resource_id, uuid);
    }

    #[test]
    fn resource_manager_request_untyped() {
        let manager = ResourceManager::new(Arc::new(FsResourceIo), Arc::new(Default::default()));
        let resource = UntypedResource::new_ok(Uuid::new_v4(), Default::default(), Stub {});
        let res = manager.register(resource.clone(), PathBuf::from("foo.txt"));
        assert!(res.is_ok());

        let res = manager.request_untyped(Path::new("foo.txt"));
        assert_eq!(res, resource);
    }

    #[test]
    fn display_for_resource_registration_error() {
        assert_eq!(
            format!("{}", ResourceRegistrationError::AlreadyRegistered),
            "A resource is already registered!"
        );
        assert_eq!(
            format!("{}", ResourceRegistrationError::InvalidState),
            "A resource was in invalid state!"
        );
        assert_eq!(
            format!("{}", ResourceRegistrationError::UnableToRegister),
            "Unable to register the resource!"
        );
    }

    #[test]
    fn debug_for_resource_registration_error() {
        assert_eq!(
            format!("{:?}", ResourceRegistrationError::AlreadyRegistered),
            "AlreadyRegistered"
        );
        assert_eq!(
            format!("{:?}", ResourceRegistrationError::InvalidState),
            "InvalidState"
        );
        assert_eq!(
            format!("{:?}", ResourceRegistrationError::UnableToRegister),
            "UnableToRegister"
        );
    }
}
