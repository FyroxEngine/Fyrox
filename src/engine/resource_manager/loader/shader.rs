use crate::{
    asset::ResourceState,
    engine::resource_manager::{
        container::event::{ResourceEvent, ResourceEventBroadcaster},
        loader::{BoxedLoaderFuture, ResourceLoader},
    },
    material::shader::{Shader, ShaderImportOptions, ShaderState},
    utils::log::{Log, MessageKind},
};
use std::{path::PathBuf, sync::Arc};

pub struct ShaderLoader;

impl ResourceLoader<Shader, ShaderImportOptions> for ShaderLoader {
    type Output = BoxedLoaderFuture;

    fn load(
        &mut self,
        shader: Shader,
        path: PathBuf,
        _default_import_options: ShaderImportOptions,
        event_broadcaster: ResourceEventBroadcaster<Shader>,
    ) -> Self::Output {
        let fut = async move {
            match ShaderState::from_file(&path).await {
                Ok(shader_state) => {
                    Log::writeln(
                        MessageKind::Information,
                        format!("Shader {:?} is loaded!", path),
                    );

                    shader.state().commit(ResourceState::Ok(shader_state));

                    event_broadcaster.broadcast(ResourceEvent::Loaded(shader));
                }
                Err(error) => {
                    Log::writeln(
                        MessageKind::Error,
                        format!("Unable to load model from {:?}! Reason {:?}", path, error),
                    );

                    shader.state().commit(ResourceState::LoadError {
                        path,
                        error: Some(Arc::new(error)),
                    });
                }
            }
        };
        Box::pin(fut)
    }
}
