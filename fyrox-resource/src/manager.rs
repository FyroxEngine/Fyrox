//! Resource manager controls loading and lifetime of resource in the engine.

use crate::untyped::ResourceKind;
use crate::{
    collect_used_resources,
    constructor::ResourceConstructorContainer,
    core::{
        append_extension,
        futures::future::join_all,
        io::FileLoadError,
        log::Log,
        make_relative_path, notify,
        parking_lot::{Mutex, MutexGuard},
        task::TaskPool,
        watcher::FileSystemWatcher,
        TypeUuidProvider,
    },
    entry::{TimedEntry, DEFAULT_RESOURCE_LIFETIME},
    event::{ResourceEvent, ResourceEventBroadcaster},
    io::{FsResourceIo, ResourceIo},
    loader::{ResourceLoader, ResourceLoadersContainer},
    options::OPTIONS_EXTENSION,
    state::{LoadError, ResourceState},
    Resource, ResourceData, TypedResourceData, UntypedResource,
};
use fxhash::{FxHashMap, FxHashSet};
use rayon::prelude::*;
use std::{
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
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
    /// The resource acccess interface
    pub resource_io: Arc<dyn ResourceIo>,

    resources: Vec<TimedEntry<UntypedResource>>,
    task_pool: Arc<TaskPool>,
    watcher: Option<FileSystemWatcher>,
}

/// See module docs.
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
        let untyped = self.state().request(path);
        let actual_type_uuid = untyped.type_uuid();
        assert_eq!(actual_type_uuid, <T as TypeUuidProvider>::type_uuid());
        Resource {
            untyped,
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
        let untyped = self.state().request(path);
        let actual_type_uuid = untyped.type_uuid();
        if actual_type_uuid == <T as TypeUuidProvider>::type_uuid() {
            Some(Resource {
                untyped,
                phantom: PhantomData::<T>,
            })
        } else {
            None
        }
    }

    /// Same as [`Self::request`], but returns untyped resource.
    pub fn request_untyped<P>(&self, path: P) -> UntypedResource
    where
        P: AsRef<Path>,
    {
        self.state().request(path)
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
        F: FnMut(&mut dyn ResourceData, &Path) -> bool,
    {
        let mut state = self.state();
        if let Some(resource) = state.find(path.as_ref()) {
            let resource_state = resource.0.lock();
            if let ResourceState::Ok(_) = resource_state.state {
                return Err(ResourceRegistrationError::AlreadyRegistered);
            }
        }

        state.unregister(path.as_ref());

        let mut header = resource.0.lock();
        header.kind.make_external(path.as_ref().to_path_buf());
        if let ResourceState::Ok(ref mut data) = header.state {
            if !on_register(&mut **data, path.as_ref()) {
                Err(ResourceRegistrationError::UnableToRegister)
            } else {
                drop(header);
                state.push(resource);
                Ok(())
            }
        } else {
            Err(ResourceRegistrationError::InvalidState)
        }
    }

    /// Attempts to move a resource from its current location to the new path.
    pub async fn move_resource(
        &self,
        resource: UntypedResource,
        new_path: impl AsRef<Path>,
        working_directory: impl AsRef<Path>,
        mut filter: impl FnMut(&UntypedResource) -> bool,
    ) -> Result<(), FileLoadError> {
        let new_path = new_path.as_ref().to_owned();
        let io = self.state().resource_io.clone();
        let existing_path = resource
            .kind()
            .into_path()
            .ok_or_else(|| FileLoadError::Custom("Cannot move embedded resource!".to_string()))?;

        let canonical_existing_path = io.canonicalize_path(&existing_path).await?;

        // Collect all resources referencing the resource.
        let resources = io
            .walk_directory(working_directory.as_ref())
            .await?
            .map(|p| self.request_untyped(p))
            .collect::<Vec<_>>();
        // Filter out all faulty resources.
        let resources_to_fix = join_all(resources)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .filter(|r| r != &resource && filter(r))
            .collect::<Vec<_>>();

        // Do the heavy work in parallel.
        let mut pairs = resources_to_fix
            .par_iter()
            .filter_map(|loaded_resource| {
                let mut guard = loaded_resource.0.lock();
                if let ResourceState::Ok(ref mut data) = guard.state {
                    let mut used_resources = FxHashSet::default();
                    (**data).as_reflect(&mut |reflect| {
                        collect_used_resources(reflect, &mut used_resources);
                    });
                    Some((loaded_resource, used_resources))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // Filter out all resources that does not have references to the moved resource.
        for (_, used_resources) in pairs.iter_mut() {
            let mut used_resources_with_references = FxHashSet::default();
            for resource in used_resources.iter() {
                // Filter out embedded resources.
                if let Some(path) = resource.kind().into_path() {
                    if let Ok(canonical_resource_path) = io.canonicalize_path(&path).await {
                        // We compare the canonical paths here to check for the same file, not for the
                        // same path. Remember that there could be any number of paths leading to the
                        // same file (i.e. "foo/bar/baz.txt" and "foo/bar/../bar/baz.txt" leads to the
                        // same file, but the paths are different).
                        if canonical_resource_path == canonical_existing_path {
                            used_resources_with_references.insert(resource.clone());
                        }
                    }
                }
            }
            *used_resources = used_resources_with_references;
        }

        for (loaded_resource, used_resources) in pairs {
            if !used_resources.is_empty() {
                for resource in used_resources {
                    resource.set_kind(ResourceKind::External(new_path.clone()));
                }

                let mut header = loaded_resource.0.lock();
                if let Some(loaded_resource_path) = header.kind.path_owned() {
                    if let ResourceState::Ok(ref mut data) = header.state {
                        // Save the resource back.
                        match data.save(&loaded_resource_path) {
                            Ok(_) => Log::info(format!(
                                "Resource {} was saved successfully!",
                                header.kind
                            )),
                            Err(err) => Log::err(format!(
                                "Unable to save {} resource. Reason: {:?}",
                                header.kind, err
                            )),
                        };
                    }
                }
            }
        }

        // Move the file with its optional import options.
        io.move_file(&existing_path, &new_path).await?;
        let options_path = append_extension(&existing_path, OPTIONS_EXTENSION);
        if io.exists(&options_path).await {
            let new_options_path = append_extension(&new_path, OPTIONS_EXTENSION);
            io.move_file(&options_path, &new_options_path).await?;
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
        }
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
                    if let Some(path) = resource.0.lock().kind.path_owned() {
                        Log::info(format!(
                            "Resource {} destroyed because it is not used anymore!",
                            path.display()
                        ));

                        self.event_broadcaster
                            .broadcast(ResourceEvent::Removed(path));
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
            if let Some(resource_path) = resource.0.lock().kind.path() {
                if resource_path == path.as_ref() {
                    return Some(&resource.value);
                }
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
            if let ResourceState::Ok(_) = resource.0.lock().state {
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
            return built_in_resource.clone();
        }

        match self.find(path.as_ref()) {
            Some(existing) => existing.clone(),
            None => {
                let path = path.as_ref().to_owned();
                let kind = ResourceKind::External(path.clone());

                if let Some(loader) = self.find_loader(path.as_ref()) {
                    let resource = UntypedResource::new_pending(kind, loader.data_type_uuid());
                    self.spawn_loading_task(path, resource.clone(), loader, false);
                    self.push(resource.clone());
                    resource
                } else {
                    let err =
                        LoadError::new(format!("There's no resource loader for {kind} resource!",));
                    UntypedResource::new_load_error(kind, err, Default::default())
                }
            }
        }
    }

    fn find_loader(&self, path: &Path) -> Option<&dyn ResourceLoader> {
        path.extension().and_then(|extension| {
            self.loaders
                .iter()
                .find(|loader| loader.supports_extension(&extension.to_string_lossy()))
        })
    }

    fn spawn_loading_task(
        &self,
        path: PathBuf,
        resource: UntypedResource,
        loader: &dyn ResourceLoader,
        reload: bool,
    ) {
        let event_broadcaster = self.event_broadcaster.clone();
        let loader_future = loader.load(path.clone(), self.resource_io.clone());
        self.task_pool.spawn_task(async move {
            match loader_future.await {
                Ok(data) => {
                    let data = data.0;

                    Log::info(format!(
                        "Resource {} was loaded successfully!",
                        path.display()
                    ));

                    // Separate scope to keep mutex locking time at minimum.
                    {
                        let mut mutex_guard = resource.0.lock();
                        assert_eq!(mutex_guard.type_uuid, data.type_uuid());
                        assert!(mutex_guard.kind.is_external());
                        mutex_guard.state.commit(ResourceState::Ok(data));
                    }

                    event_broadcaster.broadcast_loaded_or_reloaded(resource, reload);
                }
                Err(error) => {
                    Log::info(format!(
                        "Resource {} failed to load. Reason: {:?}",
                        path.display(),
                        error
                    ));

                    resource.commit_error(error);
                }
            }
        });
    }

    /// Reloads a single resource.
    pub fn reload_resource(&mut self, resource: UntypedResource) {
        let mut header = resource.0.lock();

        if !header.state.is_loading() {
            if let Some(path) = header.kind.path_owned() {
                if let Some(loader) = self.find_loader(&path) {
                    header.state.switch_to_pending_state();
                    drop(header);

                    self.spawn_loading_task(path, resource, loader, true);
                } else {
                    let msg = format!(
                        "There's no resource loader for {} resource!",
                        path.display()
                    );
                    Log::err(&msg);
                    resource.commit_error(msg)
                }
            } else {
                Log::err("Cannot reload embedded resource.")
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
        if let Some(resource) = self.find(path).cloned() {
            self.reload_resource(resource);
            true
        } else {
            false
        }
    }

    /// Forgets that a resource at the given path was ever loaded, thus making it possible to reload it
    /// again as a new instance.
    pub fn unregister(&mut self, path: &Path) {
        if let Some(position) = self
            .resources
            .iter()
            .position(|r| r.kind().path() == Some(path))
        {
            self.resources.remove(position);
        }
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;
    use std::{fs::File, time::Duration};

    use crate::loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader};

    use super::*;

    use fyrox_core::uuid::{uuid, Uuid};
    use fyrox_core::{
        reflect::{FieldInfo, Reflect},
        visitor::{Visit, VisitResult, Visitor},
        TypeUuidProvider,
    };

    #[derive(Debug, Default, Reflect, Visit)]
    struct Stub {}

    impl TypeUuidProvider for Stub {
        fn type_uuid() -> Uuid {
            uuid!("9d873ff4-3126-47e1-a492-7cd8e7168239")
        }
    }

    impl ResourceData for Stub {
        fn as_any(&self) -> &dyn std::any::Any {
            unimplemented!()
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            unimplemented!()
        }

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

        let path = PathBuf::from("test.txt");
        let type_uuid = Uuid::default();

        let cx = ResourceWaitContext {
            resources: vec![
                UntypedResource::new_pending(path.clone().into(), type_uuid),
                UntypedResource::new_load_error(path.clone().into(), Default::default(), type_uuid),
            ],
        };
        assert!(!cx.is_all_loaded());
    }

    #[test]
    fn resource_manager_state_new() {
        let state = new_resource_manager();

        assert!(state.resources.is_empty());
        assert!(state.loaders.is_empty());
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

        let path = PathBuf::from("test.txt");
        let type_uuid = Uuid::default();
        state.push(UntypedResource::new_pending(path.clone().into(), type_uuid));
        state.push(UntypedResource::new_load_error(
            path.clone().into(),
            Default::default(),
            type_uuid,
        ));
        state.push(UntypedResource::new_ok(Default::default(), Stub {}));

        assert_eq!(state.count_loaded_resources(), 1);
        assert_eq!(state.count_pending_resources(), 1);
        assert_eq!(state.count_registered_resources(), 3);
        assert_eq!(state.len(), 3);
    }

    #[test]
    fn resource_manager_state_loading_progress() {
        let mut state = new_resource_manager();

        assert_eq!(state.loading_progress(), 100);

        let path = PathBuf::from("test.txt");
        let type_uuid = Uuid::default();
        state.push(UntypedResource::new_pending(path.clone().into(), type_uuid));
        state.push(UntypedResource::new_load_error(
            path.clone().into(),
            Default::default(),
            type_uuid,
        ));
        state.push(UntypedResource::new_ok(Default::default(), Stub {}));

        assert_eq!(state.loading_progress(), 33);
    }

    #[test]
    fn resource_manager_state_find() {
        let mut state = new_resource_manager();

        assert!(state.find(Path::new("foo.txt")).is_none());

        let path = PathBuf::from("test.txt");
        let type_uuid = Uuid::default();
        let resource = UntypedResource::new_pending(path.clone().into(), type_uuid);
        state.push(resource.clone());

        assert_eq!(state.find(path), Some(&resource));
    }

    #[test]
    fn resource_manager_state_resources() {
        let mut state = new_resource_manager();

        assert_eq!(state.resources(), Vec::new());

        let path = PathBuf::from("test.txt");
        let type_uuid = Uuid::default();
        let r1 = UntypedResource::new_pending(path.clone().into(), type_uuid);
        let r2 =
            UntypedResource::new_load_error(path.clone().into(), Default::default(), type_uuid);
        let r3 = UntypedResource::new_ok(Default::default(), Stub {});
        state.push(r1.clone());
        state.push(r2.clone());
        state.push(r3.clone());

        assert_eq!(state.resources(), vec![r1.clone(), r2.clone(), r3.clone()]);
        assert!(state.iter().eq([&r1, &r2, &r3]));
    }

    #[test]
    fn resource_manager_state_destroy_unused_resources() {
        let mut state = new_resource_manager();

        state.push(UntypedResource::new_pending(
            PathBuf::from("test.txt").into(),
            Uuid::default(),
        ));
        assert_eq!(state.len(), 1);

        state.destroy_unused_resources();
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn resource_manager_state_request() {
        let mut state = new_resource_manager();
        let path = PathBuf::from("test.txt");
        let type_uuid = Uuid::default();

        let resource =
            UntypedResource::new_load_error(path.clone().into(), Default::default(), type_uuid);
        state.push(resource.clone());

        let res = state.request(path);
        assert_eq!(res, resource);

        let path = PathBuf::from("foo.txt");
        let res = state.request(path.clone());

        assert_eq!(res.kind(), ResourceKind::External(path.clone()));
        assert_eq!(res.type_uuid(), type_uuid);
        assert!(!res.is_loading());
    }

    #[test]
    fn resource_manager_state_try_reload_resource_from_path() {
        let mut state = new_resource_manager();
        state.loaders.set(Stub {});

        let resource = UntypedResource::new_load_error(
            PathBuf::from("test.txt").into(),
            Default::default(),
            Uuid::default(),
        );
        state.push(resource.clone());

        assert!(!state.try_reload_resource_from_path(Path::new("foo.txt")));

        assert!(state.try_reload_resource_from_path(Path::new("test.txt")));
        assert!(resource.is_loading());
    }

    #[test]
    fn resource_manager_state_get_wait_context() {
        let mut state = new_resource_manager();

        let resource = UntypedResource::new_ok(Default::default(), Stub {});
        state.push(resource.clone());
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
        let type_uuid = Uuid::default();

        let resource = UntypedResource::new_pending(path.clone().into(), type_uuid);
        let res = manager.register(resource.clone(), path.clone(), |_, __| true);
        assert!(res.is_err());

        let resource = UntypedResource::new_ok(Default::default(), Stub {});
        let res = manager.register(resource.clone(), path.clone(), |_, __| true);
        assert!(res.is_ok());
    }

    #[test]
    fn resource_manager_request() {
        let manager = ResourceManager::new(Arc::new(Default::default()));
        let untyped = UntypedResource::new_ok(Default::default(), Stub {});
        let res = manager.register(untyped.clone(), PathBuf::from("foo.txt"), |_, __| true);
        assert!(res.is_ok());

        let res: Resource<Stub> = manager.request(Path::new("foo.txt"));
        assert_eq!(
            res,
            Resource {
                untyped,
                phantom: PhantomData::<Stub>
            }
        );
    }

    #[test]
    fn resource_manager_request_untyped() {
        let manager = ResourceManager::new(Arc::new(Default::default()));
        let resource = UntypedResource::new_ok(Default::default(), Stub {});
        let res = manager.register(resource.clone(), PathBuf::from("foo.txt"), |_, __| true);
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
