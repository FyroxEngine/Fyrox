//! Resource manager controls loading and lifetime of resource in the engine.

use crate::{
    constructor::ResourceConstructorContainer,
    entry::{TimedEntry, DEFAULT_RESOURCE_LIFETIME},
    event::{ResourceEvent, ResourceEventBroadcaster},
    loader::ResourceLoadersContainer,
    state::ResourceState,
    task::TaskPool,
    Resource, ResourceData, UntypedResource,
};
use fxhash::FxHashMap;
use fyrox_core::{
    futures::future::join_all,
    log::Log,
    make_relative_path, notify,
    parking_lot::{Mutex, MutexGuard},
    uuid::Uuid,
    watcher::FileSystemWatcher,
    TypeUuidProvider, VecExtensions,
};
use std::path::PathBuf;
use std::{
    ffi::OsStr,
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    path::Path,
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
            if !matches!(*resource.0.lock(), ResourceState::Pending { .. }) {
                loaded_count += 1;
            }
        }
        loaded_count == self.resources.len()
    }
}

/// See module docs.
pub struct ResourceManagerState {
    /// A set of resource loaders. Use this field to register your own resource loader.
    pub loaders: ResourceLoadersContainer,
    /// Event broadcaster can be used to "subscribe" for events happening inside the container.    
    pub event_broadcaster: ResourceEventBroadcaster,
    /// A container for resource constructors.
    pub constructors_container: ResourceConstructorContainer,
    /// A set of built-in resources, that will be used to resolve references on deserialization.
    pub built_in_resources: FxHashMap<PathBuf, UntypedResource>,
    resources: Vec<TimedEntry<UntypedResource>>,
    task_pool: Arc<TaskPool>,
    watcher: Option<FileSystemWatcher>,
}

/// See module docs.
#[derive(Clone)]
pub struct ResourceManager {
    state: Arc<Mutex<ResourceManagerState>>,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// An error that may occur during texture registration.
#[derive(Debug)]
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
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ResourceManagerState::new())),
        }
    }

    /// Returns a guarded reference to internal state of resource manager.
    pub fn state(&self) -> MutexGuard<'_, ResourceManagerState> {
        self.state.lock()
    }

    /// Requests a resource of the given type located at the given path. This method is non-blocking, instead
    /// it immediately returns the typed resource wrapper. Loading of the resource is managed automatically in
    /// a separate thread (or thread pool) on PC, and JS micro-task (the same thread) on WebAssembly.
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
    pub fn request<T, P>(&self, path: P) -> Resource<T>
    where
        P: AsRef<Path>,
        T: ResourceData + TypeUuidProvider,
    {
        let untyped = self
            .state()
            .request(path, <T as TypeUuidProvider>::type_uuid());
        let actual_type_uuid = untyped.type_uuid();
        assert_eq!(actual_type_uuid, <T as TypeUuidProvider>::type_uuid());
        Resource {
            state: Some(untyped),
            phantom: PhantomData::<T>,
        }
    }

    /// Same as [`Self::request`], but returns untyped resource.
    pub fn request_untyped<P>(&self, path: P, type_uuid: Uuid) -> UntypedResource
    where
        P: AsRef<Path>,
    {
        self.state().request(path, type_uuid)
    }

    /// Saves given resources in the specified path and registers it in resource manager, so
    /// it will be accessible through it later.
    pub fn register<P, F>(
        &self,
        resource: UntypedResource,
        path: P,
        mut on_register: F,
    ) -> Result<(), ResourceRegistrationError>
    where
        P: AsRef<Path>,
        F: FnMut(&dyn ResourceData, &Path) -> bool,
    {
        let mut state = self.state();
        if state.find(path.as_ref()).is_some() {
            Err(ResourceRegistrationError::AlreadyRegistered)
        } else {
            let mut texture_state = resource.0.lock();
            match &mut *texture_state {
                ResourceState::Ok(data) => {
                    data.set_path(path.as_ref().to_path_buf());
                    if !on_register(&**data, path.as_ref()) {
                        Err(ResourceRegistrationError::UnableToRegister)
                    } else {
                        std::mem::drop(texture_state);
                        state.push(resource);
                        Ok(())
                    }
                }
                _ => Err(ResourceRegistrationError::InvalidState),
            }
        }
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
    pub(crate) fn new() -> Self {
        Self {
            resources: Default::default(),
            task_pool: Arc::new(Default::default()),
            loaders: Default::default(),
            event_broadcaster: Default::default(),
            constructors_container: Default::default(),
            watcher: None,
            built_in_resources: Default::default(),
        }
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
        self.resources.retain_mut_ext(|resource| {
            // One usage means that the resource has single owner, and that owner
            // is this container. Such resources have limited life time, if the time
            // runs out before it gets shared again, the resource will be deleted.
            if resource.value.use_count() <= 1 {
                resource.time_to_live -= dt;
                if resource.time_to_live <= 0.0 {
                    let path = resource.0.lock().path().to_path_buf();

                    Log::info(format!(
                        "Resource {} destroyed because it is not used anymore!",
                        path.display()
                    ));

                    self.event_broadcaster
                        .broadcast(ResourceEvent::Removed(path));

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

    /// Adds a new resource in the container.
    pub fn push(&mut self, resource: UntypedResource) {
        self.event_broadcaster
            .broadcast(ResourceEvent::Added(resource.clone()));

        self.resources.push(TimedEntry {
            value: resource,
            time_to_live: DEFAULT_RESOURCE_LIFETIME,
        });
    }

    /// Tries to find a resources by its path. Returns None if no resource was found.
    ///
    /// # Complexity
    ///
    /// O(n)
    pub fn find<P: AsRef<Path>>(&self, path: P) -> Option<&UntypedResource> {
        for resource in self.resources.iter() {
            if resource.0.lock().path() == path.as_ref() {
                return Some(&resource.value);
            }
        }
        None
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
            if let ResourceState::Pending { .. } = *resource.0.lock() {
                counter + 1
            } else {
                counter
            }
        })
    }

    /// Returns total amount of completely loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        self.resources.iter().fold(0, |counter, resource| {
            if let ResourceState::Ok(_) = *resource.0.lock() {
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
    pub fn request<P>(&mut self, path: P, type_uuid: Uuid) -> UntypedResource
    where
        P: AsRef<Path>,
    {
        match self.find(path.as_ref()) {
            Some(existing) => existing.clone(),
            None => {
                let resource = UntypedResource::new_pending(path.as_ref().to_owned(), type_uuid);

                self.push(resource.clone());

                self.try_spawn_loading_task(path.as_ref(), resource.clone(), false);

                resource
            }
        }
    }

    fn try_spawn_loading_task(&mut self, path: &Path, resource: UntypedResource, reload: bool) {
        if let Some(loader) = path.extension() {
            let ext_lowercase = loader.to_ascii_lowercase();
            if let Some(loader) = self.loaders.iter().find(|loader| {
                loader
                    .extensions()
                    .iter()
                    .any(|ext| OsStr::new(ext) == ext_lowercase.as_os_str())
            }) {
                self.task_pool.spawn_task(loader.load(
                    resource,
                    self.event_broadcaster.clone(),
                    reload,
                ));

                return;
            }
        }

        Log::err(format!("There's no loader registered for {:?}!", path));
    }

    /// Reloads a single resource.
    pub fn reload_resource(&mut self, resource: UntypedResource) {
        let mut state = resource.0.lock();

        if !state.is_loading() {
            let path = state.path().to_path_buf();
            state.switch_to_pending_state();
            drop(state);

            self.try_spawn_loading_task(&path, resource, true);
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
        if let Some(resource) = self.find(path).cloned() {
            self.reload_resource(resource);
            true
        } else {
            false
        }
    }
}
