//! Shader loader.

use crate::{
    asset::{
        io::ResourceIo,
        loader::{BoxedLoaderFuture, LoaderPayload, ResourceLoader},
    },
    core::{uuid::Uuid, TypeUuidProvider},
    material::shader::Shader,
};
use fyrox_resource::state::LoadError;
use std::{path::PathBuf, sync::Arc};

/// Default implementation for shader loading.
pub struct ShaderLoader;

impl ResourceLoader for ShaderLoader {
    fn extensions(&self) -> &[&str] {
        &["shader"]
    }

    fn data_type_uuid(&self) -> Uuid {
        Shader::type_uuid()
    }

    fn load(&self, path: PathBuf, io: Arc<dyn ResourceIo>) -> BoxedLoaderFuture {
        Box::pin(async move {
            let shader_state = Shader::from_file(&path, io.as_ref())
                .await
                .map_err(LoadError::new)?;
            Ok(LoaderPayload::new(shader_state))
        })
    }
}
