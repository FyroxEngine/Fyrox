use crate::engine::resource_manager::{
    container::event::ResourceEventBroadcaster, options::ImportOptions,
};
use std::{future::Future, pin::Pin};

pub mod curve;
pub mod model;
pub mod shader;
pub mod sound;
pub mod texture;

#[cfg(target_arch = "wasm32")]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = ()>>>;

#[cfg(not(target_arch = "wasm32"))]
pub type BoxedLoaderFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

pub trait ResourceLoader<T, O>
where
    T: Clone,
    O: ImportOptions,
{
    #[cfg(target_arch = "wasm32")]
    type Output: Future<Output = ()> + 'static;

    #[cfg(not(target_arch = "wasm32"))]
    type Output: Future<Output = ()> + Send + 'static;

    fn load(
        &mut self,
        resource: T,
        default_import_options: O,
        event_broadcaster: ResourceEventBroadcaster<T>,
    ) -> Self::Output;
}
