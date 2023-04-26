//! Shader loader.

use crate::{
    asset::loader::{BoxedLoaderFuture, ResourceLoader},
    core::log::Log,
    material::shader::Shader,
};
use fyrox_resource::event::ResourceEventBroadcaster;
use fyrox_resource::untyped::UntypedResource;
use std::any::Any;

/// Default implementation for shader loading.
pub struct ShaderLoader;

impl ResourceLoader for ShaderLoader {
    fn extensions(&self) -> &[&str] {
        &["shader"]
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn load(
        &self,
        shader: UntypedResource,
        event_broadcaster: ResourceEventBroadcaster,
        reload: bool,
    ) -> BoxedLoaderFuture {
        Box::pin(async move {
            let path = shader.path().to_path_buf();

            match Shader::from_file(&path).await {
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

                    shader.commit_error(path, error);
                }
            }
        })
    }
}
