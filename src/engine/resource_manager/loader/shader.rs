use crate::{
    engine::resource_manager::{
        container::event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
    },
    material::shader::{Shader, ShaderImportOptions, ShaderState},
    utils::log::Log,
};

pub struct ShaderLoader;

impl ResourceLoader<Shader, ShaderImportOptions> for ShaderLoader {
    type Output = BoxedLoaderFuture;

    fn load(
        &mut self,
        shader: Shader,
        _default_import_options: ShaderImportOptions,
        event_broadcaster: ResourceEventBroadcaster<Shader>,
    ) -> Self::Output {
        Box::pin(async move {
            let path = shader.state().path().to_path_buf();

            match ShaderState::from_file(&path).await {
                Ok(shader_state) => {
                    Log::info(format!("Shader {:?} is loaded!", path));

                    shader.state().commit_ok(shader_state);

                    event_broadcaster.broadcast_loaded(shader);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load model from {:?}! Reason {:?}",
                        path, error
                    ));

                    shader.state().commit_error(path, error);
                }
            }
        })
    }
}
