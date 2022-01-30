use crate::engine::resource_manager::loader::BoxedLoaderFuture;
use crate::{
    engine::resource_manager::{
        loader::ResourceLoader, options::try_get_import_settings, ResourceManager,
    },
    resource::model::{Model, ModelData, ModelImportOptions},
    utils::log::{Log, MessageKind},
};
use fyrox_resource::ResourceState;
use std::{path::PathBuf, sync::Arc};

pub struct ModelLoader;

impl ResourceLoader<Model, ModelImportOptions> for ModelLoader {
    type Output = BoxedLoaderFuture;

    fn load(
        &mut self,
        model: Model,
        path: PathBuf,
        default_import_options: ModelImportOptions,
        resource_manager: ResourceManager,
    ) -> Self::Output {
        let fut = async move {
            let import_options = try_get_import_settings(&path)
                .await
                .unwrap_or(default_import_options);

            match ModelData::load(&path, resource_manager, import_options).await {
                Ok(raw_model) => {
                    Log::writeln(
                        MessageKind::Information,
                        format!("Model {:?} is loaded!", path),
                    );

                    model.state().commit(ResourceState::Ok(raw_model));
                }
                Err(error) => {
                    Log::writeln(
                        MessageKind::Error,
                        format!("Unable to load model from {:?}! Reason {:?}", path, error),
                    );

                    model.state().commit(ResourceState::LoadError {
                        path,
                        error: Some(Arc::new(error)),
                    });
                }
            }
        };
        Box::pin(fut)
    }
}
