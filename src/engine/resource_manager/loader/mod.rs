use crate::engine::resource_manager::{
    container::event::ResourceEventBroadcaster, options::ImportOptions, ResourceManager
};
use std::{future::Future, pin::Pin};

pub mod curve;
pub mod model;
pub mod shader;
pub mod sound;
pub mod texture;

#[cfg(target_arch = "wasm32")]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = ()>>>;

#[cfg(target_arch = "wasm32")]
pub trait ResourceLoader<T, O>
where
    T: Clone,
    O: ImportOptions,
{
    fn load(
        &self,
        resource: T,
        default_import_options: O,
        resource_manager: ResourceManager,
        event_broadcaster: ResourceEventBroadcaster<T>,
        reload: bool,
    ) -> BoxedLoaderFuture;
}


#[cfg(not(target_arch = "wasm32"))]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

#[cfg(not(target_arch = "wasm32"))]
pub trait ResourceLoader<T, O>: Send
where
    T: Clone,
    O: ImportOptions,
{
    fn load(
        &self,
        resource: T,
        default_import_options: O,
        resource_manager: ResourceManager,
        event_broadcaster: ResourceEventBroadcaster<T>,
        reload: bool,
    ) -> BoxedLoaderFuture;
}
