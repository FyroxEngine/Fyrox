//! Resource container. It manages resource lifetime, allows you to load, re-load, wait, count
//! resources.

use crate::{
    container::{
        entry::{TimedEntry, DEFAULT_RESOURCE_LIFETIME},
        event::{ResourceEvent, ResourceEventBroadcaster},
    },
    core::{variable::InheritableVariable, VecExtensions},
    loader::ResourceLoader,
    state::ResourceState,
    task::TaskPool,
    UntypedResource,
};
use fxhash::FxHashMap;
use std::ffi::OsString;
use std::{path::Path, sync::Arc};

pub mod entry;
pub mod event;

pub(crate) trait Container {
    fn try_reload_resource_from_path(&mut self, path: &Path) -> bool;
}

/// Generic container for any resource in the engine. Main purpose of the container is to
/// track resources life time and remove unused timed-out resources. It also provides useful
/// methods to search resources, count loaded or pending, wait until all resources are loading,
/// etc.
pub struct ResourceContainer {
    resources: Vec<TimedEntry<UntypedResource>>,
    task_pool: Arc<TaskPool>,
    loader: FxHashMap<OsString, Box<dyn ResourceLoader>>,

    /// Event broadcaster can be used to "subscribe" for events happening inside the container.    
    pub event_broadcaster: ResourceEventBroadcaster,
}

impl ResourceContainer {
    pub(crate) fn new(task_pool: Arc<TaskPool>) -> Self {
        Self {
            resources: Default::default(),
            task_pool,
            loader: Default::default(),
            event_broadcaster: ResourceEventBroadcaster::new(),
        }
    }

    /// Sets the loader to load resources with.
    pub fn set_loader<S: AsRef<str>, L>(
        &mut self,
        extension: S,
        loader: L,
    ) -> Option<Box<dyn ResourceLoader>>
    where
        L: 'static + ResourceLoader,
    {
        self.loader
            .insert(OsString::from(extension.as_ref()), Box::new(loader))
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

    /// Tracks life time of resource and removes unused resources after some time of idling.
    pub fn update(&mut self, dt: f32) {
        self.resources.retain_mut_ext(|resource| {
            // One usage means that the resource has single owner, and that owner
            // is this container. Such resources have limited life time, if the time
            // runs out before it gets shared again, the resource will be deleted.
            if resource.value.use_count() <= 1 {
                resource.time_to_live -= dt;
                if resource.time_to_live <= 0.0 {
                    let path = resource.0.lock().path().to_path_buf();

                    // TODO: Use logger when it will be moved to fyrox_core.
                    println!(
                        "Resource {} destroyed because it is not used anymore!",
                        path.display()
                    );

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
    pub fn iter(&self) -> impl Iterator<Item = &UntypedResource> {
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
    pub fn request<P: AsRef<Path>>(&mut self, path: P) -> UntypedResource {
        match self.find(path.as_ref()) {
            Some(existing) => existing.clone(),
            None => {
                let resource = UntypedResource::new_pending(path.as_ref().to_owned());

                self.push(resource.clone());

                self.try_spawn_loading_task(path.as_ref(), resource.clone());

                resource
            }
        }
    }

    fn try_spawn_loading_task(&mut self, path: &Path, resource: UntypedResource) {
        if let Some(loader) = path.extension().and_then(|ext| self.loader.get(ext)) {
            self.task_pool
                .spawn_task(loader.load(resource, self.event_broadcaster.clone(), false));
        }
    }

    /// Reloads a single resource.
    pub fn reload_resource(&mut self, resource: UntypedResource) {
        let state = resource.0.lock();

        if !state.is_loading() {
            let path = state.path().to_path_buf();
            state.switch_to_pending_state();
            drop(state);

            self.try_spawn_loading_task(&path, resource);
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

    /// Tries to restore resource by making an attempt to request resource with path from existing
    /// resource instance. This method is used to restore "shallow" resources after scene
    /// deserialization.    
    pub fn try_restore_resource(&mut self, resource: &mut UntypedResource) {
        let path = resource.0.lock().path().to_path_buf();
        let new_resource = self.request(path);
        *resource = new_resource;
    }

    /// Tries to restore resource by making an attempt to request resource with path from existing
    /// resource instance. This method is used to restore "shallow" resources after scene
    /// deserialization.
    pub fn try_restore_optional_resource(&mut self, resource: &mut Option<UntypedResource>) {
        if let Some(shallow_resource) = resource.as_mut() {
            let path = shallow_resource.0.lock().path().to_path_buf();
            let new_resource = self.request(path);
            *shallow_resource = new_resource;
        }
    }

    /// Tries to restore resource by making an attempt to request resource with path from existing
    /// resource instance. This method is used to restore "shallow" resources after scene
    /// deserialization.
    pub fn try_restore_inheritable_resource(
        &mut self,
        inheritable_resource: &mut InheritableVariable<Option<UntypedResource>>,
    ) {
        if let Some(shallow_resource) = inheritable_resource.get_value_mut_silent().as_mut() {
            let new_resource = self.request(shallow_resource.0.lock().path());
            *shallow_resource = new_resource;
        }
    }
}

impl Container for ResourceContainer {
    fn try_reload_resource_from_path(&mut self, path: &Path) -> bool {
        if let Some(resource) = self.find(path).cloned() {
            self.reload_resource(resource);
            true
        } else {
            false
        }
    }
}
