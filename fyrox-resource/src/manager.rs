//! Resource manager controls loading and lifetime of resource in the engine.

use crate::constructor::ResourceConstructorContainer;
use crate::{
    container::{Container, ResourceContainer},
    state::ResourceState,
    task::TaskPool,
    Resource, ResourceData, UntypedResource,
};
use fyrox_core::uuid::Uuid;
use fyrox_core::{
    futures::future::join_all,
    make_relative_path, notify,
    parking_lot::{Mutex, MutexGuard},
    watcher::FileSystemWatcher,
    TypeUuidProvider,
};
use std::{
    fmt::{Debug, Display, Formatter},
    marker::PhantomData,
    path::Path,
    sync::Arc,
};

/// Storage of resource containers.
pub struct ContainersStorage {
    pub resources: ResourceContainer,
}

impl ContainersStorage {
    /// Wait until all resources are loaded (or failed to load).
    pub fn get_wait_context(&self) -> ResourceWaitContext {
        ResourceWaitContext {
            resources: self.resources.resources(),
        }
    }
}

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
    containers_storage: Option<ContainersStorage>,
    pub constructors_container: ResourceConstructorContainer,
    watcher: Option<FileSystemWatcher>,
}

/// See module docs.
#[derive(Clone)]
pub struct ResourceManager {
    state: Arc<Mutex<ResourceManagerState>>,
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
        let resource_manager = Self {
            state: Arc::new(Mutex::new(ResourceManagerState::new())),
        };

        let task_pool = Arc::new(TaskPool::new());

        resource_manager.state().containers_storage = Some(ContainersStorage {
            resources: ResourceContainer::new(task_pool.clone()),
        });

        resource_manager
    }

    /// Returns a guarded reference to internal state of resource manager.
    pub fn state(&self) -> MutexGuard<'_, ResourceManagerState> {
        self.state.lock()
    }

    pub fn request<T, P>(&self, path: P) -> Resource<T>
    where
        P: AsRef<Path>,
        T: ResourceData + TypeUuidProvider,
    {
        let untyped = self
            .state()
            .containers_mut()
            .resources
            .request(path, <T as TypeUuidProvider>::type_uuid());
        let actual_type_uuid = untyped.type_uuid();
        assert_eq!(actual_type_uuid, <T as TypeUuidProvider>::type_uuid());
        Resource {
            state: Some(untyped),
            phantom: PhantomData::<T>,
        }
    }

    pub fn request_untyped<P>(&self, path: P, type_uuid: Uuid) -> UntypedResource
    where
        P: AsRef<Path>,
    {
        self.state()
            .containers_mut()
            .resources
            .request(path, type_uuid)
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
        if state.containers().resources.find(path.as_ref()).is_some() {
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
                        state.containers_mut().resources.push(resource);
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
        let resources = self.state().containers_mut().resources.reload_resources();
        join_all(resources).await;
    }
}

impl ResourceManagerState {
    pub(crate) fn new() -> Self {
        Self {
            containers_storage: None,
            constructors_container: Default::default(),
            watcher: None,
        }
    }

    /// Sets resource watcher which will track any modifications in file system and forcing
    /// the manager to reload changed resources. By default there is no watcher, since it
    /// may be an undesired effect to reload resources at runtime. This is very useful thing
    /// for fast iterative development.
    pub fn set_watcher(&mut self, watcher: Option<FileSystemWatcher>) {
        self.watcher = watcher;
    }

    /// Returns a reference to resource containers storage.
    pub fn containers(&self) -> &ContainersStorage {
        self.containers_storage
            .as_ref()
            .expect("Corrupted resource manager!")
    }

    /// Returns a reference to resource containers storage.
    pub fn containers_mut(&mut self) -> &mut ContainersStorage {
        self.containers_storage
            .as_mut()
            .expect("Corrupted resource manager!")
    }

    /// Returns total amount of resources in pending state.
    pub fn count_pending_resources(&self) -> usize {
        let containers = self.containers();
        containers.resources.count_pending_resources()
    }

    /// Returns total amount of loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        let containers = self.containers();
        containers.resources.count_loaded_resources()
    }

    /// Returns total amount of registered resources.
    pub fn count_registered_resources(&self) -> usize {
        let containers = self.containers();
        containers.resources.len()
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

    /// Immediately destroys all unused resources.
    pub fn destroy_unused_resources(&mut self) {
        let containers = self.containers_mut();
        containers.resources.destroy_unused();
    }

    /// Update resource containers and do hot-reloading.
    ///
    /// Resources are removed if they're not used
    /// or reloaded if they have changed in disk.
    ///
    /// Normally, this is called from `Engine::update()`.
    /// You should only call this manually if you don't use that method.
    pub fn update(&mut self, dt: f32) {
        let containers = self.containers_mut();
        containers.resources.update(dt);

        if let Some(watcher) = self.watcher.as_ref() {
            if let Some(evt) = watcher.try_get_event() {
                if let notify::EventKind::Modify(_) = evt.kind {
                    for path in evt.paths {
                        if let Ok(relative_path) = make_relative_path(path) {
                            let containers = self.containers_mut();
                            for container in [&mut containers.resources as &mut dyn Container] {
                                if container.try_reload_resource_from_path(&relative_path) {
                                    // TODO: Use logger when it will be moved to fyrox_core.
                                    println!(
                                        "File {} was changed, trying to reload a respective resource...",
                                        relative_path.display()
                                    );

                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
