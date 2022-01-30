use crate::engine::resource_manager::loader::BoxedLoaderFuture;
use crate::{
    engine::resource_manager::{loader::ResourceLoader, ResourceManager},
    material::shader::{Shader, ShaderImportOptions, ShaderState},
    utils::log::{Log, MessageKind},
};
use fyrox_resource::ResourceState;
use std::{path::PathBuf, sync::Arc};

pub struct ShaderLoader;

impl ResourceLoader<Shader, ShaderImportOptions> for ShaderLoader {
    type Output = BoxedLoaderFuture;

    fn load(
        &mut self,
        shader: Shader,
        path: PathBuf,
        _default_import_options: ShaderImportOptions,
        _resource_manager: ResourceManager,
    ) -> Self::Output {
        let fut = async move {
            match ShaderState::from_file(&path).await {
                Ok(shader_state) => {
                    Log::writeln(
                        MessageKind::Information,
                        format!("Shader {:?} is loaded!", path),
                    );

                    shader.state().commit(ResourceState::Ok(shader_state));
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
