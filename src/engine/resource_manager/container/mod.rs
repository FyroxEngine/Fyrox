//! Resource container. It manages resource lifetime, allows you to load, re-load, wait, count
//! resources.

use crate::{
    asset::{Resource, ResourceData, ResourceLoadError, ResourceState},
    core::VecExtensions,
    engine::resource_manager::{
        container::entry::{TimedEntry, DEFAULT_RESOURCE_LIFETIME},
        loader::ResourceLoader,
        options::ImportOptions,
        task::TaskPool,
        ResourceManager,
    },
    utils::log::{Log, MessageKind},
};
use std::{
    future::Future,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

pub mod entry;

/// Generic container for any resource in the engine. Main purpose of the container is to
/// track resources life time and remove unused timed-out resources. It also provides useful
/// methods to search resources, count loaded or pending, wait until all resources are loading,
/// etc.
pub struct ResourceContainer<T, O, L>
where
    O: ImportOptions,
    L: ResourceLoader<T, O>,
{
    resources: Vec<TimedEntry<T>>,
    default_import_options: O,
    task_pool: Arc<TaskPool>,
    loader: L,
}

impl<T, R, E, O, L> ResourceContainer<T, O, L>
where
    T: Deref<Target = Resource<R, E>> + Clone + Send + Future + From<Resource<R, E>>,
    R: ResourceData,
    E: ResourceLoadError,
    O: ImportOptions,
    L: ResourceLoader<T, O>,
{
    pub(crate) fn new(task_pool: Arc<TaskPool>, loader: L) -> Self {
        Self {
            resources: Default::default(),
            default_import_options: Default::default(),
            task_pool,
            loader,
        }
    }

    /// Adds a new resource in the container.
    pub fn push(&mut self, resource: T) {
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
    pub fn find<P: AsRef<Path>>(&self, path: P) -> Option<&T> {
        for resource in self.resources.iter() {
            if resource.state().path() == path.as_ref() {
                return Some(&resource.value);
            }
        }
        None
    }

    /// Tracks life time of resource and removes unused resources after some time of idling.
    pub fn update(&mut self, dt: f32) {
        self.resources.retain_mut_ext(|resource| {
            // One usage means that the resource has single owner, and that owner
            // is this container. Such resources have limited life time, if the time
            // runs out before it gets shared again, the resource will be deleted.
            if resource.use_count() <= 1 {
                resource.time_to_live -= dt;
                if resource.time_to_live <= 0.0 {
                    Log::writeln(
                        MessageKind::Information,
                        format!(
                            "Resource {:?} destroyed because it not used anymore!",
                            resource.state().path()
                        ),
                    );

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

    /// Returns total amount of resources in the container.
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Returns true if container has no resources.
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }

    /// Creates an iterator over resources in the container.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.resources.iter().map(|entry| &entry.value)
    }

    /// Immediately destroys all resources in the container that are not used anywhere else.
    pub fn destroy_unused(&mut self) {
        self.resources
            .retain(|resource| resource.value.use_count() > 1);
    }

    /// Returns total amount of resources that still loading.
    pub fn count_pending_resources(&self) -> usize {
        self.resources.iter().fold(0, |counter, resource| {
            if let ResourceState::Pending { .. } = *resource.state() {
                counter + 1
            } else {
                counter
            }
        })
    }

    /// Returns total amount of completely loaded resources.
    pub fn count_loaded_resources(&self) -> usize {
        self.resources.iter().fold(0, |counter, resource| {
            if let ResourceState::Ok(_) = *resource.state() {
                counter + 1
            } else {
                counter
            }
        })
    }

    /// Sets default import options. Keep in mind, that actual import options could defined by a
    /// special file with additional extension `.options`.
    pub fn set_default_import_options(&mut self, options: O) {
        self.default_import_options = options;
    }

    /// Locks current thread until every resource is loaded (or failed to load).
    ///
    /// # Platform specific
    ///
    /// WASM: WebAssembly uses simple loop to wait for all resources, which means
    /// full load of single CPU core.
    pub fn wait(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            // In case of WebAssembly, spin until everything is loaded.
            loop {
                let mut loaded_count = 0;
                for resource in self.resources.iter() {
                    if !matches!(*resource.value.state(), ResourceState::Pending { .. }) {
                        loaded_count += 1;
                    }
                }
                if loaded_count == self.resources.len() {
                    break;
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::core::futures::executor::block_on(crate::core::futures::future::join_all(
                self.resources.iter().map(|t| t.value.clone()),
            ));
        }
    }

    /// Tries to load a resources at a given path.
    pub fn request<P: AsRef<Path>>(&mut self, path: P, resource_manager: ResourceManager) -> T {
        let path_ref = path.as_ref();

        if let Some(existing) = self.find(path_ref) {
            return existing.clone();
        }

        let resource = T::from(Resource::new(ResourceState::new_pending(
            path_ref.to_owned(),
        )));
        self.push(resource.clone());

        let result = resource.clone();

        self.task_pool.spawn_task(self.loader.load(
            resource,
            path_ref.to_owned(),
            self.default_import_options.clone(),
            resource_manager,
        ));

        result
    }

    /// Reloads a single resource.
    pub fn reload_resource(&mut self, resource: T, resource_manager: ResourceManager) {
        let path = resource.state().path().to_path_buf();
        let default_options = self.default_import_options.clone();
        *resource.state() = ResourceState::new_pending(path.clone());

        self.task_pool.spawn_task(self.loader.load(
            resource,
            path,
            default_options,
            resource_manager,
        ));
    }

    /// Reloads all resources in the container. Returns a list of resources that will be reloaded.
    /// You can use the list to wait until all resources are loading.
    pub fn reload_resources(&mut self, resource_manager: ResourceManager) -> Vec<T> {
        let resources = self
            .resources
            .iter()
            .map(|r| r.value.clone())
            .collect::<Vec<_>>();

        for resource in resources.iter().cloned() {
            let path = resource.state().path().to_path_buf();
            let default_import_options = self.default_import_options.clone();
            if path != PathBuf::default() {
                *resource.state() = ResourceState::new_pending(path.clone());
                self.task_pool.spawn_task(self.loader.load(
                    resource,
                    path,
                    default_import_options,
                    resource_manager.clone(),
                ));
            }
        }

        resources
    }
}
