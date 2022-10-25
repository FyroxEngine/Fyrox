//! Resource loader. It manages resource loading.

use crate::engine::resource_manager::{
    container::event::ResourceEventBroadcaster, options::ImportOptions,
};
use std::{future::Future, pin::Pin};

pub mod absm;
pub mod animation;
pub mod curve;
pub mod model;
pub mod shader;
pub mod sound;
pub mod texture;

/// Future type for resource loading. See 'ResourceLoader'.
#[cfg(target_arch = "wasm32")]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = ()>>>;

/// Trait for resource loading.
#[cfg(target_arch = "wasm32")]
pub trait ResourceLoader<T, O>
where
    T: Clone,
    O: ImportOptions,
{
    /// Loads or reloads a resource.
    fn load(
        &self,
        resource: T,
        default_import_options: O,
        event_broadcaster: ResourceEventBroadcaster<T>,
        reload: bool,
    ) -> BoxedLoaderFuture;
}

/// Future type for resource loading. See 'ResourceLoader'.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Trait for resource loading.
#[cfg(not(target_arch = "wasm32"))]
pub trait ResourceLoader<T, O>: Send
where
    T: Clone,
    O: ImportOptions,
{
    /// Loads or reloads a resource.
    fn load(
        &self,
        resource: T,
        default_import_options: O,
        event_broadcaster: ResourceEventBroadcaster<T>,
        reload: bool,
    ) -> BoxedLoaderFuture;
}
