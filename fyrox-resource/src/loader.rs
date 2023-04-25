//! Resource loader. It manages resource loading.

use crate::event::ResourceEventBroadcaster;
use crate::UntypedResource;
use std::any::Any;
use std::{future::Future, pin::Pin};

/// Future type for resource loading. See 'ResourceLoader'.
#[cfg(target_arch = "wasm32")]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = ()>>>;

/// Trait for resource loading.
#[cfg(target_arch = "wasm32")]
pub trait ResourceLoader {
    fn extensions(&self) -> &[&str];

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Loads or reloads a resource.
    fn load(
        &self,
        resource: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
    ) -> BoxedLoaderFuture;
}

/// Future type for resource loading. See 'ResourceLoader'.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Trait for resource loading.
#[cfg(not(target_arch = "wasm32"))]
pub trait ResourceLoader: Send {
    fn extensions(&self) -> &[&str];

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Loads or reloads a resource.
    fn load(
        &self,
        resource: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
    ) -> BoxedLoaderFuture;
}
