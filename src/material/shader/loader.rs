//! Shader loader.

use std::sync::Arc;

use crate::{
    asset::loader::{BoxedLoaderFuture, ResourceLoader},
    core::{log::Log, uuid::Uuid, TypeUuidProvider},
    material::shader::Shader,
};
use fyrox_resource::{event::ResourceEventBroadcaster, io::ResourceIo, untyped::UntypedResource};

/// Default implementation for shader loading.
pub struct ShaderLoader;

impl ResourceLoader for ShaderLoader {
    fn extensions(&self) -> &[&str] {
        &["shader"]
    }

    fn data_type_uuid(&self) -> Uuid {
        Shader::type_uuid()
    }

    fn load(
        &self,
        shader: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
        io: Arc<dyn ResourceIo>,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = shader.path().to_path_buf();

            match Shader::from_file(&path, io.as_ref()).await {
                Ok(shader_state) => {
                    Log::info(format!("Shader {:?} is loaded!", path));

                    shader.commit_ok(shader_state);

                    event_broadcaster.broadcast_loaded_or_reloaded(shader, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load model from {:?}! Reason {:?}",
                        path, error
                    ));

                    shader.commit_error(error);
                }
            }
        })
    }
}
