//! Model loader.

use crate::scene::node::constructor::NodeConstructorContainer;
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
use std::sync::Arc;

/// Default implementation for model loading.
pub struct ModelLoader {
    /// Resource manager to allow complex model loading.
    pub resource_manager: ResourceManager,
    /// Node constructors contains a set of constructors that allows to build a node using its
    /// type UUID.
    pub node_constructors: Arc<NodeConstructorContainer>,
}

impl ResourceLoader<Model, ModelImportOptions> for ModelLoader {
    fn load(
        &self,
        model: Model,
        default_import_options: ModelImportOptions,
        event_broadcaster: ResourceEventBroadcaster<Model>,
        reload: bool,
    ) -> BoxedLoaderFuture {
        let resource_manager = self.resource_manager.clone();
        let node_constructors = self.node_constructors.clone();

        Box::pin(async move {
            let path = model.state().path().to_path_buf();

            let import_options = try_get_import_settings(&path)
                .await
                .unwrap_or(default_import_options);

            match ModelData::load(&path, node_constructors, resource_manager, import_options).await
            {
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
