use crate::{
    engine::resource_manager::{
        container::event::ResourceEventBroadcaster,
        loader::{BoxedLoaderFuture, ResourceLoader},
        options::try_get_import_settings,
        ResourceManager,
    },
    resource::model::{Model, ModelData, ModelImportOptions},
    utils::log::Log,
};

pub struct ModelLoader {
    pub resource_manager: ResourceManager,
}

impl ResourceLoader<Model, ModelImportOptions> for ModelLoader {
    type Output = BoxedLoaderFuture;

    fn load(
        &mut self,
        model: Model,
        default_import_options: ModelImportOptions,
        event_broadcaster: ResourceEventBroadcaster<Model>,
        reload: bool,
    ) -> Self::Output {
        let resource_manager = self.resource_manager.clone();

        Box::pin(async move {
            let path = model.state().path().to_path_buf();

            let import_options = try_get_import_settings(&path)
                .await
                .unwrap_or(default_import_options);

            match ModelData::load(&path, resource_manager, import_options).await {
                Ok(raw_model) => {
                    Log::info(format!("Model {:?} is loaded!", path));

                    model.state().commit_ok(raw_model);

                    event_broadcaster.broadcast_loaded_or_reloaded(model, reload);
                }
                Err(error) => {
                    Log::err(format!(
                        "Unable to load model from {:?}! Reason {:?}",
                        path, error
                    ));

                    model.state().commit_error(path, error);
                }
            }
        })
    }
}
