use crate::engine::resource_manager::options::ImportOptions;
use std::pin::Pin;
use std::{future::Future, path::PathBuf};

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
    O: ImportOptions,
{
    #[cfg(target_arch = "wasm32")]
    type Output: Future<Output = ()> + 'static;

    #[cfg(not(target_arch = "wasm32"))]
    type Output: Future<Output = ()> + Send + 'static;

    fn load(&mut self, resource: T, path: PathBuf, default_import_options: O) -> Self::Output;
}
