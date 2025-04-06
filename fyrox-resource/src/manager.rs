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

use crate::{
    constructor::ResourceConstructorContainer,
    core::{
        append_extension,
        futures::future::join_all,
        io::FileError,
        log::Log,
        make_relative_path, notify,
        parking_lot::{Mutex, MutexGuard},
        task::TaskPool,
        watcher::FileSystemWatcher,
    },
    entry::{TimedEntry, DEFAULT_RESOURCE_LIFETIME},
    event::{ResourceEvent, ResourceEventBroadcaster},
    io::{FsResourceIo, ResourceIo},
    loader::{ResourceLoader, ResourceLoadersContainer},
    metadata::ResourceMetadata,
    options::OPTIONS_EXTENSION,
    registry::{RegistryContainerExt, ResourceRegistry, ResourceRegistryStatus},
    state::{LoadError, ResourceState},
    untyped::ResourceKind,
    Resource, TypedResourceData, UntypedResource,
};
use fxhash::FxHashMap;
use fyrox_core::{err, info, Uuid};
use std::{
    borrow::Cow,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Arc,
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
        let mut loaded_count = 0;
        for resource in self.resources.iter() {
            if !matches!(resource.0.lock().state, ResourceState::Pending { .. }) {
                loaded_count += 1;
            }
        }
        loaded_count == self.resources.len()
    }
}

/// Data source of a built-in resource.
#[derive(Clone)]
pub struct DataSource {
    /// File extension, associated with the data source.
    pub extension: Cow<'static, str>,
    /// The actual data.
    pub bytes: Cow<'static, [u8]>,
}

impl DataSource {
    pub fn new(path: &'static str, data: &'static [u8]) -> Self {
        Self {
            extension: Cow::Borrowed(
                Path::new(path)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or(""),
            ),
            bytes: Cow::Borrowed(data),
        }
    }
}

#[macro_export]
macro_rules! embedded_data_source {
    ($path:expr) => {
        $crate::manager::DataSource::new($path, include_bytes!($path))
    };
}

#[derive(Clone)]
pub struct UntypedBuiltInResource {
    pub id: PathBuf,
    /// Initial data, from which the resource is created from.
    pub data_source: Option<DataSource>,
    /// Ready-to-use ("loaded") resource.
    pub resource: UntypedResource,
}

pub struct BuiltInResource<T>
where
    T: TypedResourceData,
{
    pub id: PathBuf,
    /// Initial data, from which the resource is created from.
    pub data_source: Option<DataSource>,
    /// Ready-to-use ("loaded") resource.
    pub resource: Resource<T>,
}

impl<T: TypedResourceData> Clone for BuiltInResource<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            data_source: self.data_source.clone(),
            resource: self.resource.clone(),
        }
    }
}

impl<T: TypedResourceData> BuiltInResource<T> {
    pub fn new<F>(id: impl AsRef<Path>, data_source: DataSource, make: F) -> Self
    where
        F: FnOnce(&[u8]) -> Resource<T>,
    {
        let resource = make(&data_source.bytes);
        Self {
            id: id.as_ref().to_path_buf(),
            resource,
            data_source: Some(data_source),
        }
    }

    pub fn new_no_source(id: impl AsRef<Path>, resource: Resource<T>) -> Self {
        Self {
            id: id.as_ref().to_path_buf(),
            data_source: None,
            resource,
        }
    }

    pub fn resource(&self) -> Resource<T> {
        self.resource.clone()
    }
}

impl<T: TypedResourceData> From<BuiltInResource<T>> for UntypedBuiltInResource {
    fn from(value: BuiltInResource<T>) -> Self {
        Self {
            id: value.id,
            data_source: value.data_source,
            resource: value.resource.into(),
        }
    }
}

#[derive(Default, Clone)]
pub struct BuiltInResourcesContainer {
    inner: FxHashMap<PathBuf, UntypedBuiltInResource>,
}

impl BuiltInResourcesContainer {
    pub fn add<T>(&mut self, resource: BuiltInResource<T>)
    where
        T: TypedResourceData,
    {
        self.add_untyped(resource.id.clone(), resource.into())
    }

    pub fn add_untyped(&mut self, id: PathBuf, resource: UntypedBuiltInResource) {
        self.inner.insert(id, resource);
    }

    pub fn find_by_uuid(&self, uuid: Uuid) -> Option<&UntypedBuiltInResource> {
        self.inner
            .values()
            .find(|r| r.resource.resource_uuid() == Some(uuid))
    }
}

impl Deref for BuiltInResourcesContainer {
    type Target = FxHashMap<PathBuf, UntypedBuiltInResource>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for BuiltInResourcesContainer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
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
        }
    }
}

impl ResourceManager {
    /// Creates a resource manager with default settings and loaders.
    pub fn new(task_pool: Arc<TaskPool>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ResourceManagerState::new(task_pool))),
        }
    }

    /// Returns a guarded reference to internal state of resource manager.
    pub fn state(&self) -> MutexGuard<'_, ResourceManagerState> {
        self.state.lock()
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
        let mut state = self.state();

        assert!(state
            .loaders
            .lock()
            .is_extension_matches_type::<T>(path.as_ref()));

        Resource {
            untyped: state.request(path),
            phantom: PhantomData::<T>,
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
        if state
            .loaders
            .lock()
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

    pub fn resource_path(&self, resource: &UntypedResource) -> Option<PathBuf> {
        self.state().resource_path(resource)
    }

    /// Same as [`Self::request`], but returns untyped resource.
    pub fn request_untyped<P>(&self, path: P) -> UntypedResource
    where
        P: AsRef<Path>,
    {
        self.state().request(path)
    }

    pub fn update_and_load_registry(&self, path: impl AsRef<Path>) {
        self.state().update_and_load_registry(path);
    }

    pub fn add_loader<T: ResourceLoader>(&self, loader: T) -> Option<T> {
        self.state().add_loader(loader)
    }

    /// Saves given resources in the specified path and registers it in resource manager, so
    /// it will be accessible through it later.
    pub fn register(
        &self,
        resource: UntypedResource,
        path: impl AsRef<Path>,
    ) -> Result<(), ResourceRegistrationError> {
        self.state().register(resource, path)
    }

    /// Attempts to move a resource from its current location to the new path.
    pub async fn move_resource(
        &self,
        resource: &UntypedResource,
        new_path: impl AsRef<Path>,
    ) -> Result<(), FileError> {
        let resource_uuid = resource
            .resource_uuid()
            .ok_or_else(|| FileError::Custom("Unable to move non-loaded resource!".to_string()))?;

        let new_path = new_path.as_ref().to_owned();
        let io = self.state().resource_io.clone();
        let registry = self.state().resource_registry.clone();
        let existing_path = registry
            .lock()
            .uuid_to_path(resource_uuid)
            .map(|path| path.to_path_buf())
            .ok_or_else(|| FileError::Custom("Cannot move embedded resource!".to_string()))?;

        // Move the file with its optional import options and mandatory metadata.
        io.move_file(&existing_path, &new_path).await?;

        assert_eq!(
            registry
                .lock()
                .register(resource_uuid, new_path.clone())
                .as_ref(),
            Some(&existing_path)
        );

        let options_path = append_extension(&existing_path, OPTIONS_EXTENSION);
        if io.exists(&options_path).await {
            let new_options_path = append_extension(&new_path, OPTIONS_EXTENSION);
            io.move_file(&options_path, &new_options_path).await?;
        }

        let metadata_path = append_extension(&existing_path, ResourceMetadata::EXTENSION);
        if io.exists(&metadata_path).await {
            let new_metadata_path = append_extension(&new_path, ResourceMetadata::EXTENSION);
            io.move_file(&metadata_path, &new_metadata_path).await?;
        }

        Ok(())
    }

    /// Reloads all loaded resources. Normally it should never be called, because it is **very** heavy
    /// method! This method is asynchronous, it uses all available CPU power to reload resources as
    /// fast as possible.
    pub async fn reload_resources(&self) {
        let resources = self.state().reload_resources();
        join_all(resources).await;
    }
}

impl ResourceManagerState {
    pub(crate) fn new(task_pool: Arc<TaskPool>) -> Self {
        Self {
            resources: Default::default(),
            task_pool,
            loaders: Default::default(),
            event_broadcaster: Default::default(),
            constructors_container: Default::default(),
            watcher: None,
            built_in_resources: Default::default(),
            // Use the file system resource io by default
            resource_io: Arc::new(FsResourceIo),
            resource_registry: Arc::new(Mutex::new(ResourceRegistry::default())),
        }
    }

    pub fn update_and_load_registry(&self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();

        info!(
            "Trying to load or update the registry at {}...",
            path.display()
        );

        let resource_io = self.resource_io.clone();
        let resource_registry = self.resource_registry.clone();
        #[allow(unused_variables)]
        let excluded_folders = resource_registry.lock().excluded_folders.clone();
        let registry_status = resource_registry.lock().status.clone();
        registry_status.mark_as_loading();
        #[allow(unused_variables)]
        let task_loaders = self.loaders.clone();
        self.task_pool.spawn_task(async move {
            // Try to update the registry first.
            // Wasm is an exception, because it does not have a file system.
            #[cfg(not(target_arch = "wasm32"))]
            {
                let new_data = ResourceRegistry::scan(
                    resource_io.clone(),
                    task_loaders,
                    &path,
                    excluded_folders,
                )
                .await;
                if let Err(error) = new_data.save(&path, &*resource_io).await {
                    err!(
                        "Unable to write the resource registry at the {} path! Reason: {:?}",
                        path.display(),
                        error
                    )
                }
                let mut lock = resource_registry.lock();
                lock.set_container(new_data);
                registry_status.mark_as_loaded();

                info!(
                    "Resource registry was updated and written to {} successfully!",
                    path.display()
                );
            }

            // WASM can only try to load the existing registry.
            #[cfg(target_arch = "wasm32")]
            {
                // Then load the registry.
                match crate::registry::RegistryContainer::load_from_file(&path, &*resource_io).await
                {
                    Ok(registry) => {
                        let mut lock = resource_registry.lock();
                        lock.set_container(registry);

                        registry_status.mark_as_loaded();

                        info!(
                            "Resource registry was loaded from {} successfully!",
                            path.display()
                        );
                    }
                    Err(error) => {
                        err!("Unable to load resource registry! Reason: {:?}.", error);
                    }
                };
            }
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
                    let registry = self.resource_registry.lock();
                    let resource_uuid = resource.resource_uuid();
                    if let Some(path) =
                        resource_uuid.and_then(|resource_uuid| registry.uuid_to_path(resource_uuid))
                    {
                        Log::info(format!(
                            "Resource {} destroyed because it is not used anymore!",
                            path.display()
                        ));

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

        if let Some(watcher) = self.watcher.as_ref() {
            if let Some(evt) = watcher.try_get_event() {
                if let notify::EventKind::Modify(_) = evt.kind {
                    for path in evt.paths {
                        if let Ok(relative_path) = make_relative_path(path) {
                            if self.try_reload_resource_from_path(&relative_path) {
                                Log::info(format!(
                                    "File {} was changed, trying to reload a respective resource...",
                                    relative_path.display()
                                ));

                                break;
                            }
                        }
                    }
                }
            }
        }
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
            .find(|entry| entry.value.resource_uuid() == Some(uuid))
            .map(|entry| &entry.value)
    }

    pub fn find_by_path(&self, path: &Path) -> Option<&UntypedResource> {
        let registry = self.resource_registry.lock();
        self.resources.iter().find_map(|entry| {
            let header = entry.value.0.lock();
            if let ResourceState::Ok { resource_uuid, .. } = header.state {
                if registry.uuid_to_path(resource_uuid) == Some(path) {
                    return Some(&entry.value);
                }
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
        self.resources.iter().fold(0, |counter, resource| {
            if let ResourceState::Pending { .. } = resource.0.lock().state {
                counter + 1
            } else {
                counter
            }
        })
    }

    /// Returns total amount of completely loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        self.resources.iter().fold(0, |counter, resource| {
            if let ResourceState::Ok { .. } = resource.0.lock().state {
                counter + 1
            } else {
                counter
            }
        })
    }

    /// Returns a set of resource handled by this container.
    pub fn resources(&self) -> Vec<UntypedResource> {
        self.resources.iter().map(|t| t.value.clone()).collect()
    }

    /// Tries to load a resources at a given path.
    pub fn request<P>(&mut self, path: P) -> UntypedResource
    where
        P: AsRef<Path>,
    {
        if let Some(built_in_resource) = self.built_in_resources.get(path.as_ref()) {
            return built_in_resource.resource.clone();
        }

        let path = ResourceRegistry::prepare_path(path);

        self.find_or_load(path)
    }

    fn find_by_resource_path(&self, path_to_search: &PathBuf) -> Option<&UntypedResource> {
        self.resources
            .iter()
            .find(|entry| {
                let header = entry.value.0.lock();
                match header.state {
                    ResourceState::Pending { ref path, .. }
                    | ResourceState::LoadError { ref path, .. } => path == path_to_search,
                    ResourceState::Ok { resource_uuid, .. } => {
                        self.resource_registry.lock().uuid_to_path(resource_uuid)
                            == Some(path_to_search)
                    }
                }
            })
            .map(|entry| &entry.value)
    }

    fn find_or_load(&mut self, path: PathBuf) -> UntypedResource {
        match self.find_by_resource_path(&path) {
            Some(existing) => existing.clone(),
            None => {
                let resource = UntypedResource::new_pending(path.clone(), ResourceKind::External);
                self.add_resource_and_notify(resource.clone());
                self.spawn_loading_task(path, resource.clone(), false);
                resource
            }
        }
    }

    fn spawn_loading_task(&self, path: PathBuf, resource: UntypedResource, reload: bool) {
        let event_broadcaster = self.event_broadcaster.clone();
        let loaders = self.loaders.clone();
        let registry = self.resource_registry.clone();
        let io = self.resource_io.clone();
        let registry_status = registry.lock().status.clone();

        self.task_pool.spawn_task(async move {
            // Wait until the registry is fully loaded.
            let registry_status = registry_status.await;
            if registry_status == ResourceRegistryStatus::Unknown {
                resource.commit_error(
                    path.clone(),
                    LoadError::new("The resource registry is unavailable!".to_string()),
                );
                return;
            }

            // Try to find a loader for the resource.
            let loader_future = loaders
                .lock()
                .loader_for(&path)
                .map(|loader| loader.load(path.clone(), io));

            if let Some(loader_future) = loader_future {
                match loader_future.await {
                    Ok(data) => {
                        let data = data.0;

                        match registry.lock().path_to_uuid(&path) {
                            Some(resource_uuid) => {
                                let mut mutex_guard = resource.0.lock();

                                assert!(mutex_guard.kind.is_external());

                                mutex_guard.state.commit(ResourceState::Ok {
                                    data,
                                    resource_uuid,
                                });

                                drop(mutex_guard);

                                event_broadcaster.broadcast_loaded_or_reloaded(resource, reload);

                                Log::info(format!(
                                    "Resource {} was loaded successfully!",
                                    path.display()
                                ));
                            }
                            None => {
                                let error = format!(
                                    "Resource {} failed to load. The path was not found \
                                        in the registry!",
                                    path.display(),
                                );

                                resource.commit_error(path, error);
                            }
                        }
                    }
                    Err(error) => {
                        Log::info(format!(
                            "Resource {} failed to load. Reason: {:?}",
                            path.display(),
                            error
                        ));

                        resource.commit_error(path, error);
                    }
                }
            } else {
                resource.commit_error(
                    path.clone(),
                    LoadError::new(format!(
                        "There's no resource loader for {} resource!",
                        path.display()
                    )),
                )
            }
        });
    }

    pub fn resource_path(&self, resource: &UntypedResource) -> Option<PathBuf> {
        let header = resource.0.lock();
        if let ResourceState::Ok { resource_uuid, .. } = header.state {
            let registry = self.resource_registry.lock();
            registry.uuid_to_path_buf(resource_uuid)
        } else {
            None
        }
    }

    pub fn add_loader<T: ResourceLoader>(&self, loader: T) -> Option<T> {
        self.loaders.lock().set(loader)
    }

    /// Saves given resources in the specified path and registers it in resource manager, so
    /// it will be accessible through it later.
    pub fn register(
        &mut self,
        resource: UntypedResource,
        path: impl AsRef<Path>,
    ) -> Result<(), ResourceRegistrationError> {
        let path = path.as_ref().to_owned();

        let resource_uuid = resource
            .resource_uuid()
            .ok_or(ResourceRegistrationError::InvalidState)?;

        if self.find_by_uuid(resource_uuid).is_some() {
            return Err(ResourceRegistrationError::AlreadyRegistered);
        }

        let mut resource_header = resource.0.lock();
        resource_header.kind.make_external();
        if let ResourceState::Ok { resource_uuid, .. } = resource_header.state {
            self.resource_registry.lock().register(resource_uuid, path);
            drop(resource_header);
            self.add_resource_and_notify(resource);
            Ok(())
        } else {
            Err(ResourceRegistrationError::InvalidState)
        }
    }

    /// Reloads a single resource.
    pub fn reload_resource(&mut self, resource: UntypedResource) {
        let header = resource.0.lock();
        match header.state {
            ResourceState::Pending { .. } => {
                // The resource is loading already.
            }
            ResourceState::LoadError { ref path, .. } => {
                let path = path.clone();
                drop(header);
                self.spawn_loading_task(path, resource, true)
            }
            ResourceState::Ok { resource_uuid, .. } => {
                let path = self
                    .resource_registry
                    .lock()
                    .uuid_to_path_buf(resource_uuid);
                drop(header);
                if let Some(path) = path {
                    self.spawn_loading_task(path, resource, true);
                } else {
                    err!(
                        "Unable to reload a {resource_uuid} resource, because it is not \
                    registered in the resource registry and its path is unknown! "
                    );
                }
            }
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

    /// Tries to reload a resource at the given path.
    pub fn try_reload_resource_from_path(&mut self, path: &Path) -> bool {
        let mut registry = self.resource_registry.lock();
        if let Some(uuid) = registry.unregister_path(path) {
            drop(registry);
            if let Some(resource) = self.find_by_uuid(uuid).cloned() {
                self.reload_resource(resource);
                return true;
            }
        }
        false
    }

    /// Forgets that a resource at the given path was ever loaded, thus making it possible to reload it
    /// again as a new instance.
    pub fn unregister(&mut self, path: &Path) {
        if let Some(uuid) = self.resource_registry.lock().unregister_path(path) {
            if let Some(position) = self
                .resources
                .iter()
                .position(|entry| entry.value.resource_uuid() == Some(uuid))
            {
                self.resources.remove(position);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
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

    #[derive(Debug, Default, Reflect, Visit)]
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
        ResourceManagerState::new(Arc::new(Default::default()))
    }

    #[test]
    fn resource_wait_context_is_all_loaded() {
        assert!(ResourceWaitContext::default().is_all_loaded());

        let cx = ResourceWaitContext {
            resources: vec![
                UntypedResource::new_pending(Default::default(), ResourceKind::External),
                UntypedResource::new_load_error(ResourceKind::External, Default::default()),
            ],
        };
        assert!(!cx.is_all_loaded());
    }

    #[test]
    fn resource_manager_state_new() {
        let state = new_resource_manager();

        assert!(state.resources.is_empty());
        assert!(state.loaders.lock().is_empty());
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
        let mut state = new_resource_manager();

        assert_eq!(state.count_loaded_resources(), 0);
        assert_eq!(state.count_pending_resources(), 0);
        assert_eq!(state.count_registered_resources(), 0);
        assert_eq!(state.len(), 0);

        assert_eq!(
            state.register(
                UntypedResource::new_pending(Default::default(), ResourceKind::External),
                "foo.bar",
            ),
            Err(ResourceRegistrationError::InvalidState)
        );
        assert_eq!(
            state.register(
                UntypedResource::new_load_error(ResourceKind::External, Default::default()),
                "foo.bar",
            ),
            Err(ResourceRegistrationError::InvalidState)
        );
        assert_eq!(
            state.register(
                UntypedResource::new_ok(Uuid::new_v4(), Default::default(), Stub {}),
                "foo.bar",
            ),
            Ok(())
        );

        assert_eq!(state.count_registered_resources(), 1);
        assert_eq!(state.len(), 1);
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
        let manager = ResourceManager::new(Arc::new(Default::default()));

        assert!(manager.state.lock().is_empty());
        assert!(manager.state().is_empty());
    }

    #[test]
    fn resource_manager_register() {
        let manager = ResourceManager::new(Arc::new(Default::default()));
        let path = PathBuf::from("test.txt");

        let resource = UntypedResource::new_pending(Default::default(), ResourceKind::External);
        let res = manager.register(resource.clone(), path.clone());
        assert!(res.is_err());

        let resource = UntypedResource::new_ok(Uuid::new_v4(), ResourceKind::External, Stub {});
        let res = manager.register(resource.clone(), path.clone());
        assert!(res.is_ok());
    }

    #[test]
    fn resource_manager_request_untyped() {
        let manager = ResourceManager::new(Arc::new(Default::default()));
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
